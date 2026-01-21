# cchooked - Claude Code Hooks Engine

[![CI](https://github.com/1gy/cchooked/actions/workflows/ci.yml/badge.svg)](https://github.com/1gy/cchooked/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/1gy/cchooked)](https://github.com/1gy/cchooked/releases/latest)
[![License](https://img.shields.io/github/license/1gy/cchooked)](https://github.com/1gy/cchooked/blob/main/LICENSE)
[![GitHub](https://img.shields.io/badge/GitHub-1gy%2Fcchooked-blue?logo=github)](https://github.com/1gy/cchooked)

Claude Code の hooks 機能向けルールベースエンジン。TOML 設定ファイルで宣言的にルールを定義し、コマンドのブロック、変換、ログ記録などを行います。

## 特徴

- **宣言的なルール定義** - TOML ファイルでシンプルに設定
- **柔軟なマッチング** - 正規表現によるコマンド・ファイルパス・ブランチの条件指定
- **3種類のアクション** - block, run, log
- **変数展開** - `${command}`, `${file_path}`, `${file_dir}`, `${workspace_root}`, `${branch}` などを利用可能

## クイックスタート

### 方法1: GitHub Releaseからインストール（推奨）

最新のビルド済みバイナリをダウンロードしてインストールします。

```bash
# Linux x86_64 用バイナリをダウンロード
curl -L -o cchooked https://github.com/1gy/cchooked/releases/latest/download/cchooked-linux-x86_64

# チェックサムを検証
curl -L -o cchooked.sha256 https://github.com/1gy/cchooked/releases/latest/download/cchooked-linux-x86_64.sha256
sha256sum -c cchooked.sha256

# インストール
chmod +x cchooked
mv cchooked ~/.local/bin/

# クリーンアップ
rm cchooked.sha256
```

### 方法2: cargo install でインストール

Rust のパッケージマネージャーを使用してインストールします。

```bash
cargo install --git https://github.com/1gy/cchooked
```

### 方法3: ソースからビルド

リポジトリをクローンしてビルドします。

```bash
# リポジトリをクローン
git clone https://github.com/1gy/cchooked.git
cd cchooked

# ビルド
cargo build --release

# パスの通った場所にコピー
cp target/release/cchooked ~/.local/bin/
```

### 設定ファイルの準備

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
