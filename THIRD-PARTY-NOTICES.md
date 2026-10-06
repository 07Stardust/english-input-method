# 第三方来源与许可说明

English Input Method 基于青简 Qingjian，代码 GPL-3.0-or-later，保留原作者历史及署名。Windows 产品图标为本项目新绘制的 E 图标；青简原始名称与 logo 不属于代码授权。

随包数据及运行时分别适用其原有许可，不将所有数据统一标为 GPL：

- 中文词库：规范汉字表、现代汉语常用词表转录、THUOCL（MIT）、Unihan（Unicode License v3）；转录来源未全部附开放许可。详见 assets/lexicon/README.md 与 docs/design/landscape.md。
- 本地释义：青简离线 LLM 生成词库，GPL-3.0-or-later；生成内容可能有错，见 assets/glossary/README.md。
- 语言模型：中文维基 CC BY-SA 4.0 与 LCCC MIT；随上游锁定数据包取得，来源见 docs/design/landscape.md。
- 英文词表：ESDB、CSpell、typos 等，各自许可文本在 assets/lexicon/05_english/sources/。
- 词汇等级：CEFR-J（要求署名）、Octanove（CC BY-SA 4.0）、Tanos JLPT（CC BY，经 elzup 整理）；见 assets/levels/README.md。
- Emoji：Unicode CLDR，Unicode License v3；原文见 assets/emoji/LICENSE-unicode.txt。
- 五笔码表：sxjudya/rime-wubi86-jidian，Apache-2.0；见 assets/wubi/LICENSE 与 README.md。
- CET 词目：NEEA 官方 2016 大纲的事实词目、星标与页码整理。没有已确认的开放数据许可；不复制官方正文、样卷或 PDF，不作 GPL 再许可。释义另来自本地青简数据，来源及计数限制见 assets/study/cet/README.md。
- Windows App Runtime / WinUI 与 Rust 依赖按其各自许可提供；依赖版本固定在 Cargo.lock，自包含运行时来自构建流程使用的官方 NuGet。

完整来源表与署名见 docs/design/landscape.md、各 assets 子目录 README/LICENSE；许可证声明应随数据保持，公开源代码包含相应来源文件。
