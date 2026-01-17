# cchooked - Claude Code Hooks Engine

![CI](https://github.com/1gy/cchooked/actions/workflows/ci.yml/badge.svg)
![Release](https://img.shields.io/github/v/release/1gy/cchooked)
![License](https://img.shields.io/github/license/1gy/cchooked)
[![GitHub](https://img.shields.io/badge/GitHub-1gy%2Fcchooked-blue?logo=github)](https://github.com/1gy/cchooked)

Claude Code の hooks 機能向けルールベースエンジン。TOML 設定ファイルで宣言的にルールを定義し、コマンドのブロック、変換、ログ記録などを行います。

## 特徴

- **宣言的なルール定義** - TOML ファイルでシンプルに設定
- **柔軟なマッチング** - 正規表現によるコマンド・ファイルパス・ブランチの条件指定
- **4種類のアクション** - block, transform, run, log
- **変数展開** - `${command}`, `${file_path}`, `${branch}` などを利用可能

## クイックスタート

```bash
# ビルド
cargo build --release

# パスの通った場所にコピー
cp target/release/cchooked ~/.local/bin/
```

`.claude/settings.local.json` に hooks を設定:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [{ "type": "command", "command": "cchooked PreToolUse" }]
      }
    ]
  }
}
```

`.claude/hooks-rules.toml` にルールを記述:

```toml
[rules.prefer-bun]
event = "PreToolUse"
matcher = "Bash"
action = "block"
message = "bun を使用してください"
when.command = "^npm\\s"
```

## ドキュメント

- **[利用ガイド](docs/usage.md)** - インストール、設定、使用例の詳細
- **[技術仕様](docs/spec.md)** - 開発者向け内部仕様

## ライセンス

MIT License
