# GeekZip

GeekZip 是使用 Rust、GPUI 与 gpui-component 构建的原生归档工具，面向 macOS。

## 功能

- ZIP、7Z、TAR、GZ、BZ2、XZ、ZSTD、LZ4 解压
- ZIP、TAR、TAR.GZ、TAR.BZ2、TAR.XZ 压缩
- 自动密码本、递归解压、文件夹批量解压和目录自动解压
- Magic Bytes 格式识别
- Normal 简洁模式与 Pro 专业模式
- 实时 CPU、GPU、内存与线程监控
- 点阵网格、分段进度和科技风原生界面

## 运行

    cargo run -p geekzip-gpui

命令行版本：

    cargo run -p geekzip-cli -- --help

## 构建

    cargo build --release -p geekzip-gpui -p geekzip-cli

密码本保存在 ~/.geekzip/passwords.json。

## 状态

v0.2.0 完成从 React/Tauri 到 Rust GPUI 原生界面的迁移。当前发布包针对
macOS Apple Silicon，未进行 Apple 公证签名。
