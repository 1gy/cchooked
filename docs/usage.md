# cchooked 利用ガイド

このドキュメントでは cchooked のインストールから設定、使用例までを詳しく説明します。

## 目次

- [インストール](#インストール)
- [Claude Code hooks への設定方法](#claude-code-hooks-への設定方法)
- [設定ファイル（hooks-rules.toml）の詳細](#設定ファイルhooks-rulestomlの詳細)
- [アクション説明](#アクション説明)
- [変数展開](#変数展開)
- [よくある使用例](#よくある使用例)
- [トラブルシューティング](#トラブルシューティング)

## インストール

### ソースからビルド

```bash
# リポジトリをクローン
git clone https://github.com/1gy/cchooked.git
cd cchooked

# ビルド
cargo build --release

# パスの通った場所にコピー（オプション）
cp target/release/cchooked ~/.local/bin/
```

## Claude Code hooks への設定方法

`.claude/settings.local.json` に以下の設定を追加します：

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "cchooked PreToolUse"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "cchooked PostToolUse"
          }
        ]
      }
    ]
  }
}
```

## 設定ファイル（hooks-rules.toml）の詳細

設定ファイルは `.claude/hooks-rules.toml` に配置します。

### 基本構造

```toml
[rules.ルール名]
event = "PreToolUse"      # または "PostToolUse"
matcher = "Bash"          # ツール名（正規表現可、| で OR）
action = "block"          # アクションの種類
message = "メッセージ"     # block 時のメッセージ
priority = 10             # 評価順序（高い順、デフォルト: 0）
when.command = "^npm\\s"  # マッチ条件（正規表現）
```

### 必須フィールド

| フィールド | 説明 |
|-----------|------|
| `event` | `"PreToolUse"` または `"PostToolUse"` |
| `matcher` | ツール名パターン（正規表現可、`\|` で OR） |
| `action` | `"block"` / `"transform"` / `"run"` / `"log"` |

### オプションフィールド

| フィールド | デフォルト | 説明 |
|-----------|-----------|------|
| `priority` | 0 | 評価順序（高い値が優先） |
| `message` | - | block 時のメッセージ |
| `when.command` | - | コマンドの正規表現パターン |
| `when.file_path` | - | ファイルパスの正規表現パターン |
| `when.branch` | - | Git ブランチ名の正規表現パターン |
| `transform.command` | - | `[pattern, replacement]` 形式 |
| `command` | - | run アクション用コマンド |
| `on_error` | "ignore" | `"ignore"` / `"fail"` |
| `log_file` | - | ログ出力先（log アクションでは必須） |
| `log_format` | "text" | `"text"` / `"json"` |
| `working_dir` | `${file_dir}` | run アクションのコマンド実行ディレクトリ（`file_path` が指定されていない場合は cchooked の CWD） |

### when 条件の評価

```toml
# 配列内の値は OR 評価（node または npm または npx）
when.command = ["^node\\s", "^npm\\s", "^npx\\s"]

# 異なるフィールド間は AND 評価（branch=main かつ file_path=src/**）
when.branch = "main"
when.file_path = "^src/.*"
```

## アクション説明

### 1. block - コマンドをブロック

指定した条件にマッチした場合、コマンドの実行をブロックしてメッセージを表示します。

```toml
[rules.prefer-bun]
event = "PreToolUse"
matcher = "Bash"
action = "block"
message = """
この環境では bun を使用してください:
- node -> bun
- npm install -> bun install
- npx -> bunx
"""
when.command = "^(node|npm|npx|yarn)\\s"
```

### 2. transform - コマンドを変換

マッチしたコマンドを別のコマンドに自動変換します。

```toml
[rules.auto-replace-npm]
event = "PreToolUse"
matcher = "Bash"
action = "transform"
when.command = "^npm\\s"
transform.command = ["^npm", "bun"]
```

この設定により、`npm install express` は自動的に `bun install express` に変換されます。

### 3. run - コマンドを実行

ファイル編集後にフォーマッターを実行するなど、追加のコマンドを実行します。

```toml
[rules.format-typescript]
event = "PostToolUse"
matcher = "Edit|Write"
action = "run"
command = "prettier --write ${file_path}"
on_error = "fail"
when.file_path = ".*\\.tsx?$"
```

**working_dir オプション:**

`working_dir` でコマンドを実行するディレクトリを指定できます。デフォルトは `${file_dir}`（file_path の親ディレクトリ）です。`file_path` が指定されていない場合は cchooked の CWD で実行されます。

```toml
# デフォルト: file_path の親ディレクトリで実行
[rules.auto-format]
event = "PostToolUse"
matcher = "Edit|Write"
action = "run"
command = "bun run format"

# 明示的にワークスペースルートで実行
[rules.build]
event = "PostToolUse"
matcher = "Write"
action = "run"
command = "bun run build"
working_dir = "${workspace_root}"

# サブディレクトリを指定
[rules.frontend-lint]
event = "PostToolUse"
matcher = "Edit|Write"
action = "run"
command = "bun run lint"
working_dir = "${workspace_root}/frontend"
```

**on_error オプション:**

| 値 | 動作 |
|-----|------|
| `ignore` | エラーを無視して続行（デフォルト） |
| `fail` | エラーメッセージを表示して処理を中断 |

### 4. log - ログを記録

コマンド実行をファイルに記録します。`log_file` は必須です。

```toml
[rules.audit-log]
event = "PreToolUse"
matcher = "Bash"
action = "log"
log_file = "~/.claude/command-history.log"  # 必須
log_format = "json"
```

## 変数展開

以下の変数が `message`、`command`、`transform` 内で使用可能です：

| 変数 | 説明 | 例 |
|------|------|-----|
| `${command}` | Bash コマンド全体 | `npm install express` |
| `${file_path}` | ファイルパス | `/src/index.ts` |
| `${file_dir}` | file_path の親ディレクトリ | `/src` |
| `${workspace_root}` | CLAUDE_PROJECT_DIR 環境変数の値（未設定時は cchooked の CWD） | `/home/user/project` |
| `${tool_name}` | ツール名 | `Bash`, `Edit`, `Write` |
| `${branch}` | 現在の Git ブランチ | `main`, `feature/new` |

## よくある使用例

### npm を bun に置き換える

```toml
[rules.prefer-bun]
event = "PreToolUse"
matcher = "Bash"
action = "block"
priority = 10
message = "bun を使用してください（npm -> bun, npx -> bunx）"
when.command = "^(npm|npx|yarn)\\s"

[rules.auto-bun]
event = "PreToolUse"
matcher = "Bash"
action = "transform"
priority = 20
when.command = "^npm\\s"
transform.command = ["^npm", "bun"]
```

### 環境ファイルを保護する

```toml
[rules.protect-env]
event = "PreToolUse"
matcher = "Edit|Write"
action = "block"
message = ".env ファイルは直接編集できません"
when.file_path = ".*\\.env.*"
```

### main ブランチでの編集を防止

```toml
[rules.protect-main-branch]
event = "PreToolUse"
matcher = "Edit|Write"
action = "block"
message = "main ブランチでは編集できません。別のブランチで作業してください。"
when.branch = "main"
```

### TypeScript ファイルを自動フォーマット

```toml
[rules.format-typescript]
event = "PostToolUse"
matcher = "Edit|Write"
action = "run"
command = "prettier --write ${file_path}"
on_error = "fail"
when.file_path = ".*\\.tsx?$"
```

## CLI の使い方

```bash
# 基本的な使用法（Claude Code hooks から自動で呼び出される）
cchooked <event>

# 手動でテスト
echo '{"tool_name":"Bash","tool_input":{"command":"npm install"}}' | cchooked PreToolUse

# 設定ファイルパス指定
cchooked PreToolUse --config /path/to/hooks-rules.toml

# バージョン表示
cchooked --version

# ヘルプ
cchooked --help
```

## トラブルシューティング

### ルールがマッチしない

1. **event の確認**: `PreToolUse` と `PostToolUse` を間違えていないか確認
2. **正規表現の確認**: TOML では `\` をエスケープする必要があります（`\\s` など）
3. **手動テスト**: 以下のコマンドで動作確認
   ```bash
   echo '{"tool_name":"Bash","tool_input":{"command":"npm install"}}' | cchooked PreToolUse
   ```

### 設定ファイルが読み込まれない

- デフォルトの配置場所は `.claude/hooks-rules.toml`
- `--config` オプションで明示的にパスを指定可能
- 設定ファイルのパースエラーは exit code 1 で stderr に出力

### transform が適用されない

- transform は stdout に JSON を出力します
- `transform.command` は `[pattern, replacement]` 形式の配列で指定
- パターンは正規表現として評価されます

### run コマンドのエラーが表示されない

- `on_error = "ignore"`（デフォルト）では exit 0 のため Claude Code に表示されません
- エラーを確認したい場合は `on_error = "fail"` を使用

### Git ブランチが取得できない

- `.git` ディレクトリが存在しない場合、`${branch}` は空文字になります
- `when.branch` 条件は Git リポジトリ外では常にマッチしません

## 関連ドキュメント

- [技術仕様](spec.md) - 開発者向け内部仕様
