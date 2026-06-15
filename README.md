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

## 安装

macOS 桌面应用：

    brew install --cask mcxen/geekzip/geekzip

命令行工具：

    brew install mcxen/geekzip/geekzip-cli

## 构建

    cargo build --release -p geekzip-gpui -p geekzip-cli

密码本保存在 ~/.geekzip/passwords.json。

## 状态

v0.2.1 加入 GZ 霓虹应用图标、Tactical Terminal 视觉主题、动态资源曲线和
Homebrew 安装。当前发布包针对 macOS Apple Silicon，未进行 Apple 公证签名。
