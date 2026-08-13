//! Extract plain text from Office Open XML files so `read_file` works without Word.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use zip::ZipArchive;

use crate::core::tools::error::ToolError;

pub fn is_office_document(path: &Path) -> bool {
    matches!(
        extension(path).as_deref(),
        Some("docx" | "dotx" | "xlsx" | "xlsm" | "pptx")
    )
}

pub fn extract_office_plain_text(path: &Path) -> Result<String, ToolError> {
    match extension(path).as_deref() {
        Some("docx" | "dotx") => extract_docx(path),
        Some("xlsx" | "xlsm") => extract_xlsx(path),
        Some("pptx") => extract_pptx(path),
        _ => Err(ToolError::new("not an office document")),
    }
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
}

fn open_zip(path: &Path) -> Result<ZipArchive<File>, ToolError> {
    let file = File::open(path)?;
    ZipArchive::new(file).map_err(|e| ToolError::new(format!("open office zip: {e}")))
}

fn zip_names(archive: &mut ZipArchive<File>) -> Result<Vec<String>, ToolError> {
    let mut names = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let name = archive
            .by_index(i)
            .map_err(|e| ToolError::new(format!("zip entry: {e}")))?
            .name()
            .to_string();
        names.push(name);
    }
    Ok(names)
}

fn read_entry(archive: &mut ZipArchive<File>, name: &str) -> Result<String, ToolError> {
    let mut entry = archive
        .by_name(name)
        .map_err(|e| ToolError::new(format!("missing {name}: {e}")))?;
    let mut xml = String::new();
    entry
        .read_to_string(&mut xml)
        .map_err(|e| ToolError::new(format!("read {name}: {e}")))?;
    Ok(xml)
}

fn extract_docx(path: &Path) -> Result<String, ToolError> {
    let mut archive = open_zip(path)?;
    let names = zip_names(&mut archive)?;
    if !names.iter().any(|n| n == "word/document.xml") {
        return Err(ToolError::new("not a valid .docx (missing word/document.xml)"));
    }
    let mut ordered: Vec<String> = names
        .into_iter()
        .filter(|n| {
            n == "word/document.xml"
                || ((n.starts_with("word/header") || n.starts_with("word/footer"))
                    && n.ends_with(".xml"))
        })
        .collect();
    ordered.sort_by(|a, b| rank_docx(a).cmp(&rank_docx(b)).then(a.cmp(b)));
    let mut parts = Vec::new();
    for name in ordered {
        let xml = read_entry(&mut archive, &name)?;
        let text = wml_plain_text(&xml);
        if !text.is_empty() {
            parts.push(text);
        }
    }
    let out = parts.join("\n");
    if out.trim().is_empty() {
        Ok("(no extractable text in this .docx — it may be scanned images)".into())
    } else {
        Ok(out)
    }
}

fn rank_docx(name: &str) -> u8 {
    if name == "word/document.xml" {
        0
    } else if name.starts_with("word/header") {
        1
    } else {
        2
    }
}

fn extract_xlsx(path: &Path) -> Result<String, ToolError> {
    let mut archive = open_zip(path)?;
    let names = zip_names(&mut archive)?;
    let mut parts = Vec::new();
    if names.iter().any(|n| n == "xl/sharedStrings.xml") {
        let xml = read_entry(&mut archive, "xl/sharedStrings.xml")?;
        let text = tagged_plain_text(&xml, "t");
        if !text.is_empty() {
            parts.push(text);
        }
    }
    let mut sheets: Vec<String> = names
        .into_iter()
        .filter(|n| n.starts_with("xl/worksheets/") && n.ends_with(".xml"))
        .collect();
    sheets.sort();
    for name in sheets {
        let xml = read_entry(&mut archive, &name)?;
        let text = tagged_plain_text(&xml, "v");
        if !text.is_empty() {
            parts.push(text);
        }
    }
    let out = parts.join("\n");
    if out.trim().is_empty() {
        Ok("(no extractable text in this spreadsheet)".into())
    } else {
        Ok(out)
    }
}

fn extract_pptx(path: &Path) -> Result<String, ToolError> {
    let mut archive = open_zip(path)?;
    let names = zip_names(&mut archive)?;
    let mut slides: Vec<String> = names
        .into_iter()
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .collect();
    slides.sort();
    let mut parts = Vec::new();
    for name in slides {
        let xml = read_entry(&mut archive, &name)?;
        let text = tagged_plain_text(&xml, "t");
        if !text.is_empty() {
            parts.push(text);
        }
    }
    let out = parts.join("\n\n");
    if out.trim().is_empty() {
        Ok("(no extractable text in this .pptx)".into())
    } else {
        Ok(out)
    }
}

/// WordprocessingML → paragraphs / table cells.
pub(crate) fn wml_plain_text(xml: &str) -> String {
    let mut out = String::new();
    let mut rest = xml;
    while let Some(rel) = rest.find('<') {
        rest = &rest[rel..];
        if is_end_tag(rest, "tr") {
            if out.ends_with('\t') {
                out.pop();
            }
            push_newline(&mut out);
        } else if is_end_tag(rest, "p") {
            push_newline(&mut out);
        } else if is_end_tag(rest, "tc") {
            if out.ends_with('\n') {
                out.pop();
            }
            out.push('\t');
        } else if is_empty_or_start(rest, "tab") {
            out.push('\t');
        } else if is_empty_or_start(rest, "br") || is_empty_or_start(rest, "cr") {
            push_newline(&mut out);
        } else if let Some(text) = take_inner(rest, "t") {
            out.push_str(&unescape_xml(text.0));
            rest = text.1;
            continue;
        }
        rest = skip_tag(rest);
    }
    collapse_blank_lines(&out)
}

/// Collect inner text of every `<…:local>` / `<local>` element, one per line.
fn tagged_plain_text(xml: &str, local: &str) -> String {
    let mut out = String::new();
    let mut rest = xml;
    while let Some(rel) = rest.find('<') {
        rest = &rest[rel..];
        if let Some(text) = take_inner(rest, local) {
            let piece = unescape_xml(text.0);
            if !piece.is_empty() {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&piece);
            }
            rest = text.1;
            continue;
        }
        rest = skip_tag(rest);
    }
    out
}

fn is_end_tag(rest: &str, local: &str) -> bool {
    let after = match rest.strip_prefix("</") {
        Some(v) => v,
        None => return false,
    };
    let after = strip_ns(after);
    after.starts_with(local)
        && matches!(
            after.as_bytes().get(local.len()),
            Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'/')
        )
}

fn is_empty_or_start(rest: &str, local: &str) -> bool {
    let after = match rest.strip_prefix('<') {
        Some(v) => v,
        None => return false,
    };
    if after.starts_with('/') {
        return false;
    }
    let after = strip_ns(after);
    after.starts_with(local)
        && matches!(
            after.as_bytes().get(local.len()),
            Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'/')
        )
}

fn strip_ns(tag: &str) -> &str {
    match tag.find(':') {
        Some(i) if i < 12 => &tag[i + 1..],
        _ => tag,
    }
}

fn take_inner<'a>(rest: &'a str, local: &str) -> Option<(&'a str, &'a str)> {
    if !is_empty_or_start(rest, local) {
        return None;
    }
    let gt = rest.find('>')?;
    if rest.as_bytes().get(gt.saturating_sub(1)) == Some(&b'/') {
        return None;
    }
    let inner = &rest[gt + 1..];
    let close_rel = find_close(inner, local)?;
    Some((&inner[..close_rel], &inner[close_rel..]))
}

fn find_close(inner: &str, local: &str) -> Option<usize> {
    let mut search = inner;
    let mut offset = 0usize;
    while let Some(rel) = search.find("</") {
        let abs = offset + rel;
        if is_end_tag(&inner[abs..], local) {
            return Some(abs);
        }
        offset = abs + 2;
        search = &inner[offset..];
    }
    None
}

fn skip_tag(rest: &str) -> &str {
    match rest.find('>') {
        Some(i) => &rest[i + 1..],
        None => "",
    }
}

fn push_newline(out: &mut String) {
    if out.ends_with('\n') {
        return;
    }
    if !out.is_empty() {
        out.push('\n');
    }
}

fn collapse_blank_lines(text: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut blank = 0u8;
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            blank = blank.saturating_add(1);
            if blank <= 1 && !lines.is_empty() {
                lines.push(String::new());
            }
            continue;
        }
        blank = 0;
        lines.push(trimmed.to_string());
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

fn unescape_xml(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        if let Some(end) = rest.find(';') {
            let ent = &rest[..=end];
            out.push_str(match ent {
                "&amp;" => "&",
                "&lt;" => "<",
                "&gt;" => ">",
                "&quot;" => "\"",
                "&apos;" => "'",
                "&#9;" | "&#x9;" => "\t",
                "&#10;" | "&#xA;" | "&#xa;" => "\n",
                "&#13;" | "&#xD;" | "&#xd;" => "\r",
                _ => {
                    out.push('&');
                    rest = &rest[1..];
                    continue;
                }
            });
            rest = &rest[end + 1..];
        } else {
            out.push_str(rest);
            return out;
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    #[test]
    fn extracts_paragraphs_and_table_cells() {
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
              <w:p><w:r><w:t>Hello</w:t></w:r></w:p>
              <w:p><w:r><w:t xml:space="preserve">World &amp; Co</w:t></w:r></w:p>
              <w:tbl>
                <w:tr>
                  <w:tc><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc>
                  <w:tc><w:p><w:r><w:t>B</w:t></w:r></w:p></w:tc>
                </w:tr>
              </w:tbl>
            </w:body>
          </w:document>"#;
        let text = wml_plain_text(xml);
        assert!(text.contains("Hello"));
        assert!(text.contains("World & Co"));
        assert!(text.contains("A\tB"), "got: {text:?}");
    }

    #[test]
    fn does_not_treat_ppr_as_paragraph() {
        let xml = r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Title</w:t></w:r></w:p>"#;
        assert_eq!(wml_plain_text(xml), "Title");
    }

    #[test]
    fn extracts_docx_from_zip() {
        let xml = r#"<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>投标文件</w:t></w:r></w:p></w:body></w:document>"#;
        let bytes = pack_docx(xml);
        let dir = std::env::temp_dir().join(format!("anya-docx-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.docx");
        std::fs::write(&path, bytes).unwrap();
        let text = extract_docx(&path).expect("extract");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(text.contains("投标文件"), "got: {text:?}");
    }

    fn pack_docx(document_xml: &str) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(cursor);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("word/document.xml", opts).unwrap();
        zip.write_all(document_xml.as_bytes()).unwrap();
        zip.finish().unwrap().into_inner()
    }
}
