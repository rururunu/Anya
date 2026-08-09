# 发布与远程更新

Anya 使用 [Tauri Updater](https://v2.tauri.app/plugin/updater/) 从 GitHub Releases 拉取更新。客户端配置的地址为：

`https://github.com/rururunu/Anya/releases/latest/download/latest.json`

## `latest.json` 在哪？

| 位置               | 说明                                                                              |
| ------------------ | --------------------------------------------------------------------------------- |
| **Git 仓库**       | **没有**常驻的 `latest.json`，一般不提交到版本库                                  |
| **本地生成**       | 运行 `pnpm release:json` 后输出到 **`release/latest.json`**（见下文脚本）         |
| **GitHub Release** | 手动或 CI 上传的附件，**文件名必须是 `latest.json`**                              |
| **用户客户端访问** | `releases/latest/download/latest.json`（指向**最新非预发布** Release 里的该文件） |

构建 MSI 后，签名文件与安装包在：

- `src-tauri/target/release/bundle/msi/Anya_<version>_x64.msi`
- `src-tauri/target/release/bundle/msi/Anya_<version>_x64.msi.sig`

`.msi.sig` 里的内容会写入 `latest.json` 的 `platforms.windows-x86_64.signature` 字段（填**文件内容**，不是 URL）。

---

## 密钥（首次配置）

```powershell
pnpm tauri signer generate -w "$env:USERPROFILE\.tauri\anya.key" --ci
```

- **私钥**：`%USERPROFILE%\.tauri\anya.key`（勿提交、勿泄露；丢失后无法为已安装用户发更新）
- **公钥**：已写入 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`；本地还可放在 `src-tauri/updater.pubkey`（已在 `.gitignore`）

GitHub Actions 需在仓库 Secrets 中配置：

| Secret                               | 内容                                     |
| ------------------------------------ | ---------------------------------------- |
| `TAURI_SIGNING_PRIVATE_KEY`          | 私钥文件完整内容                         |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥密码（未设密码可留空）               |
| `ANYA_UPDATER_PUBKEY`                | 公钥（可选；公钥已在 `tauri.conf.json`） |

---

## 手动发布（推荐流程）

### 1. 改版本号

同步修改：

- `src-tauri/tauri.conf.json` → `version`
- `package.json` → `version`

### 2. 本地构建并签名

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.tauri\anya.key" -Raw
pnpm tauri:build
```

产物目录：`src-tauri/target/release/bundle/msi/`。

### 3. 生成本地 `latest.json`

```powershell
pnpm release:json -- --tag v0.2.6 --notes "更新说明（可选）"
```

生成文件：**`release/latest.json`**。

也可手写，格式示例：

```json
{
  "version": "0.2.6",
  "notes": "更新说明",
  "pub_date": "2026-08-06T08:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "url": "https://github.com/rururunu/Anya/releases/download/v0.2.6/Anya_0.2.6_x64.msi",
      "signature": "<Anya_0.2.6_x64.msi.sig 文件的全部内容>"
    }
  }
}
```

### 4. 在 GitHub 创建 Release

1. 打开 [Releases](https://github.com/rururunu/Anya/releases) → **Draft a new release**
2. Tag：`v0.2.6`（与 `--tag` 一致）
3. 填写 Release 说明（与 `notes` 可相同，给用户看）
4. 上传 **3 个文件**：
   - `Anya_0.2.6_x64.msi`
   - `Anya_0.2.6_x64.msi.sig`
   - `latest.json`（来自 `release/latest.json`，**文件名保持 `latest.json`**）
5. 发布（不要勾 Pre-release，否则 `latest` 不会指向它）

### 5. 验证

1. 浏览器打开：`https://github.com/rururunu/Anya/releases/latest/download/latest.json`，确认 `version` 与 `url`、`signature` 正确
2. 打开 Anya → **设置 → 关于** → **检测更新**，或等待工作台右上角更新按钮

---

## CI 自动发布（可选）

仓库已包含 `.github/workflows/release.yml`：推送 `v*` 标签时自动构建、签名并上传 Release。

```powershell
git tag v0.2.6
git push origin v0.2.6
```

配置好上述 Secrets 后，无需再手动写 Release 正文或上传 MSI；`latest.json` 由 `tauri-action` 生成并上传。

若你**只用手动发布**，可以不使用该 workflow，或仅在需要时打 tag 触发。

---

## 常见问题

**Q：只上传 MSI，不上传 `latest.json` 可以吗？**  
不行。Updater 只认 `latest.json` 里的版本与签名，用户只能去 Releases 页面手动下载。

**Q：旧版 Release（如 v0.1.3）没有 `latest.json` 怎么办？**  
只有用新流程发布的版本才能被应用内更新；旧用户需先手动安装一版带 Updater 的新包，之后才能自动更新。

**Q：`signature` 填什么？**  
填 `.msi.sig` 文件的**全文**（一行 base64 文本），不是文件路径也不是下载链接。

**相关文档：** [技术架构](./architecture-overview.zh-CN.md) · [文档索引](./README.zh-CN.md)
