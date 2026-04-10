# RPing

高性能多目标 Ping 工具，支持同时 Ping 1000+ 个 IP。

**Powered by byff** | © 2026 byff

## 功能

- 多目标并发 Ping（支持 1000+ IP）
- 支持 IP、CIDR、域名输入
- 实时结果表格，所有列支持排序
- 从 TXT / Excel 导入 IP 列表，支持拖拽
- Excel 自动识别 IP 列
- 导出结果到 Excel / 插入结果到源表
- 可配置超时、包大小、间隔、并发数
- 配置文件本地持久化
- 现代深色科技风 GUI

## 平台支持

| 平台 | 架构 | 状态 |
|------|------|------|
| Windows | x86_64 | ✅ |
| Linux (麒麟/Kylin) | aarch64 | ✅ |
| Linux | x86_64 | ✅ |

## 编译

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 编译当前平台
cargo build --release

# 交叉编译全平台
./build-all.sh
```

## 技术栈

- Rust + Tokio（异步并发）
- egui / eframe（GUI）
- surge-ping（ICMP）
- calamine + rust_xlsxwriter（Excel 读写）
