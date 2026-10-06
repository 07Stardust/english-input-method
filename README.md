# English Input Method

Windows 英语学习输入法，基于 [青简 Qingjian](https://github.com/qingjian-team/qingjian) 开发。在正常输入中文时显示关联英文词，配合本地词书、词卡和拼写练习，适合六级备考与日常学习。

**当前版本：v0.1.5-alpha.1，公开测试版。** 首版面向 Windows 10/11 x64，包含供 32 位应用使用的输入法 DLL。安装器需要管理员权限；目前未签名，首次安装后可能需要注销登录。编译及自动测试通过不等于所有应用场景均已验收。

## 下载与安装

从 [本项目 Releases](https://github.com/07Stardust/english-input-method/releases) 下载 `english-input-method-0.1.5-alpha.1-windows-x86_64-setup.exe` 和 `SHA256SUMS.txt`。源码 ZIP 不是安装包。

1. 核对 SHA256 后运行 EXE，按向导安装。
2. 按 **Win + 空格**切换到 **English Input Method**；列表未出现时注销后重新登录。
3. 在开始菜单 **English Input Method → English Input Method 设置**打开设置。
4. 进入 **词书与学习**，导入安装目录 `assets/study/cet/` 的 CSV，选择词书并点击 **只使用选中词书**。
5. 开启学习开关；六级备考开启考试模式，考试标签填 `CET6`。等级筛选留空，并包含未知等级。

普通用户首次安装不会自动导入词书；安装包随附词书文件，需要按上述步骤确认导入。输入有对应关联的中文候选时会呈现英文提示，无匹配时继续正常中文输入。关闭设置窗口后后台进程继续运行。

PowerShell 7 核对下载文件：

```powershell
Get-FileHash .\english-input-method-0.1.5-alpha.1-windows-x86_64-setup.exe -Algorithm SHA256
```

## 功能

- 保留中文拼音输入和英文提示，学习逻辑离线运行。
- 支持 CSV、TSV、Excel `.xlsx` 词书；列映射、导入预览、多词书启停、更新保留进度。
- 考试模式限定词书及筛选范围；日常模式允许内置通用释义兜底。
- 学习开关关闭提示与学习统计，普通中文输入继续；隐私开关暂停记录与云请求，两者独立。
- 状态条、菜单和录制快捷键支持切换；设置热加载通常约 1 秒，无需重启后台进程。
- 释义词卡、看中文写英文、收藏、错词与到期复习；默认每天新增 20 词，间隔 1、3、7、14、30 天。
- 所有学习数据保存在本机，提供导出与明确的删除入口。

## 六级词书与数据来源

| 随附词书 | 数量 | 范围 |
| --- | ---: | --- |
| `cet6-neea-2016-star.csv` | 1,282 | 官方大纲六级新增星标词，展开拼写并合并同形词 |
| `cet6-neea-2016-full.csv` | 5,404 | 基础与六级主要词目合并提取版，不含单列派生词 |

英文词目与星标来自 [教育部教育考试院 2016 大纲](https://cet.neea.edu.cn/xhtml1/folder/16113/1588-1.htm)。中文释义、输入关联来自青简本地离线生成词库，**不是官方释义**，可能需要人工修正。CEFR 等级与六级标签分别记录，未知等级不能推定为六级。

官方注明 5,418 词目；合并提取版与此计数仍有未完全解释的差异，**不宣称无遗漏全集或考试覆盖率**。官方词目没有已确认的开放数据许可，不将其标为 GPL；不附原始 PDF、样卷或正文。详见 [词书来源、限制与校验](assets/study/cet/README.md)及 [第三方数据说明](THIRD-PARTY-NOTICES.md)。

## 隐私与测试范围

- 默认关闭云服务与输入原文日志；输入法不需要账号。
- Windows 云密钥使用当前用户 DPAPI 保护；诊断导出排除输入历史和密钥。
- 用户数据目录：`%LOCALAPPDATA%\EnglishInputMethod`，与上游产品独立。
- 隐私开关与输入框的私密状态叠加；设置页跨进程切换存在热加载延迟，不承诺瞬时生效。
- 上游自动更新通道已关闭，当前需要手动下载本项目新版本。
- Windows CI 已验证格式、严格 Clippy 与 488 项相关测试；依赖审计的已知漏洞数量和限制见验证报告。自动测试不构成“绝对安全”结论。
- 普通文本框、浏览器、VS Code、密码框、系统搜索及 AppContainer 的安装后验收仍待完成。测试包关闭 uiAccess，部分系统界面的候选窗口显示仍需验证。

首次使用建议先在普通文本编辑器检查输入及开关行为。完整说明：[词书与学习](docs/user/english-study.md)；验证范围：[实现报告](docs/notes/english-implementation-verification.md)。

## 开发与构建

源码保留上游完整历史，Windows 学习功能放在共享核心，平台负责界面和系统交互。构建依赖 Rust 1.96.0、Windows MSVC 工具链及 Inno Setup 7；生成数据依据锁文件取回并校验 SHA256。

GitHub Actions 的 **Build Windows test installer** 为手动构建，只上传测试安装包，不自动安装或发布，也不使用上游发布凭据。完整构建建议交给 CI，避免本地 debug 缓存占用大量磁盘。

```powershell
# PowerShell 7，完整 Windows 构建
./tools/release/data-fetch.ps1
./apps/windows/installer/build.ps1
```

开发约定见 [CLAUDE.md](CLAUDE.md)与 [贡献说明](docs/contributing.md)。问题反馈请使用 [本仓库 Issues](https://github.com/07Stardust/english-input-method/issues)。

## 许可证与署名

代码沿用 [GPL-3.0-or-later](LICENSE)，保留青简作者署名与历史；源代码和对应版本标签随 Release 提供。上游名称和 logo 不属于代码授权，本产品使用独立名称与 Windows 图标。上游介绍存档见 [来源存档](docs/notes/upstream-readme.md)。各数据文件依各自许可及来源说明使用，不合并进代码许可证。
