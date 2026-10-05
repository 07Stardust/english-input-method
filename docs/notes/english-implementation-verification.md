# English Input Method 实现验证报告

日期：2026-10-05。源码：D:/codex/english-tick/work/qingjian-audit。私人仓库：https://github.com/07Stardust/english-input-method。

## 当前结论

本地已实现共享学习核心、Windows 词书/复习设置页、两个开关、独立安装身份、IPC/密钥/日志及云隐私修复。格式检查、严格 Clippy 和 487 项相关测试通过。依赖升级后 RustSec 报告为 0 项已知漏洞；此结论不代表不存在漏洞。

远程私人仓库已建立，CLI 与插件实际账户同为 07Stardust（数字 ID 234716685）；CLI 旧账户显示来自改名后的本地元数据。GitHub 因令牌缺少 workflow scope 拒绝包含工作流的首次基线推送。追加 scope 尚待用户答复；未上传基线或开发提交，未运行私人仓库 CI。

## 已验证证据

| 项目 | 结果与范围 |
| --- | --- |
| 工具链 | Rust 1.96.0 Windows GNU；本地测试禁用 uiAccess，未安装输入法 |
| 格式 | cargo fmt --all -- --check 通过 |
| 静态检查 | core、learning、platform、predict、Windows server/tsf/settings 的 all-targets Clippy -D warnings 通过 |
| Core | 303 项：含学习关闭、隐私上下文清理、学习/云请求拦截回归 |
| Learning | 26 项：CSV、TSV/GBK/UTF16、规范 XLSX、多义/等价答案、预览错误/重复、更新保留进度、失败回滚、考试范围、间隔和配额 |
| Platform | 49 项及 custom-phrases 集成 3 项：协议归属、关闭撤销、旧协议/畸形帧、DPAPI、配置迁移/脱敏、解析错误不泄露密钥 |
| Predict | 16 项：配置密钥 Debug 脱敏、云任务取消与迟到结果处理等 |
| Server | 6 项单元、67 项引擎集成、2 项内存 IPC、1 项真实 Windows 管道集成 |
| TSF | 14 项按键/布局/程序路径单元测试 |
| 真实管道 | 双端身份核验、正常中文输入、另一连接冒用会话被拒绝、127 连接上限及释放后恢复；在沙箱外使用当前 Windows 用户令牌运行 |
| 导入安全 | 解压体积、稀疏单元格/维度、公式、外部链接及宏拒绝路径；输入候选只查询预先构建的索引 |
| 依赖审计 | cargo-audit 0.22.2，数据库提交 ef6173cbc5c50ec8166f9a5b28f07834144373ee，0 项已知漏洞，未配置忽略项 |
| 打包脚本 | PowerShell 7 语法检查通过；暂存目录递归删除前验证绝对路径在仓库内 |

真实管道测试证明当前用户普通令牌下的连接行为，不等同于跨 Windows 登录会话、其他用户或 AppContainer 的实际验证。

## 依赖修复与保留提示

升级 calamine 0.31.0 至 0.36.1，直接 quick-xml 0.38 至 0.41；锁文件不再含有存在漏洞的 quick-xml 0.38.4。修复 [RUSTSEC-2026-0194](https://rustsec.org/advisories/RUSTSEC-2026-0194.html) 和 [RUSTSEC-2026-0195](https://rustsec.org/advisories/RUSTSEC-2026-0195.html) 的 XML 解析拒绝服务风险。

保留两项停止维护提示：paste（RUSTSEC-2024-0436，经 candle/gemm 链路）及 ttf-parser（RUSTSEC-2026-0192，字体渲染链路）。未以隐藏提示或广泛替换渲染依赖来宣称安全完成。

## 未完成验收与具体限制

1. 本机已准备 MSVC Rust 目标，但没有可用 Visual Studio MSVC 链接器。WinUI 自包含运行时需要 MSVC 构建；因此尚未生成或验证 Windows 安装包。已准备手动 Windows CI 构建流程，待推送授权后运行并取回产物。
2. 未安装或注册 IME；普通文本框、浏览器、VS Code、密码框、系统搜索、焦点切换与实际候选窗口未进行应用验收。未完成 AppContainer 或其他 Windows 用户/登录会话的运行测试。
3. 状态条/菜单隐私开关直接作用于引擎；设置页通过配置热加载作用于输入进程（轮询间隔 1 秒），云连接测试轮询间隔 100 ms。跨进程立即取消、短暂开关切换仍需加强与专项验收，不把轮询称为瞬时响应。
4. 管道有读取/连接限制，但同步写入尚未加入专门写超时；连接耗尽回归不能代表完整拒绝服务防护。
5. 词书/练习 JSON 导出入口与词汇曝光统计分开，尚未提供统一恢复入口；演示书只有 5 个词，不代表六级全集或覆盖率。
6. 未执行上游发布、安装、系统注册或证书信任操作。测试安装包默认关闭 uiAccess，商店应用/系统搜索的候选窗口置顶需另行验证。

## 交付资料与复现

词书模板：assets/study/template.csv、template.tsv、template.xlsx；演示书：demo-cet6.csv。用户说明：docs/user/english-study.md。

本地证据文件位于 D:/codex/english-tick/outputs：
- implementation-tests.log：487 项测试结果。
- implementation-clippy.log：最终严格检查日志。
- dependency-audit.json / dependency-audit.log：完整依赖审计结果。

所有 shell 命令使用 PowerShell 7。复现时设置 CARGO_TARGET_DIR 指向独立构建目录，QINGJIAN_UIACCESS=0，并使用 cargo +1.96.0-x86_64-pc-windows-gnu。真实 Windows 管道集成需要普通用户令牌，受限沙箱令牌无法替代安装后的应用验收。