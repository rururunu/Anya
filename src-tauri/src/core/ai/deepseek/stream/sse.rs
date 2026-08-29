pub(super) fn next_sse_line(buffer: &mut String) -> Option<String> {
    let newline_index = buffer.find('\n')?;
    let mut line = buffer.drain(..=newline_index).collect::<String>();
    if line.ends_with('\n') {
        line.pop();
    }
    if line.ends_with('\r') {
        line.pop();
    }
    Some(line)
}
