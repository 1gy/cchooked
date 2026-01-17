# cchooked 技術仕様書

このドキュメントは開発者向けの技術仕様です。利用方法については [利用ガイド](usage.md) を参照してください。

## 入出力フォーマット

### 入力仕様（stdin）

Claude Code から受け取る JSON 形式。

#### PreToolUse イベント

```json
{
  "tool_name": "Bash",
  "tool_input": {
    "command": "npm install express",
    "description": "Install express package"
  }
}
```

#### PostToolUse イベント

```json
{
  "tool_name": "Write",
  "tool_input": {
    "file_path": "/path/to/file.ts",
    "content": "..."
  },
  "tool_response": "..."
}
```

### 出力仕様

#### block アクション

```
exit code: 2
stderr: {message}
stdout: (空)
```

#### transform アクション

```
exit code: 0
stdout: JSON (下記スキーマ参照)
stderr: (空)
```

**出力 JSON スキーマ:**

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow",
    "updatedInput": {
      "command": "bun install express"
    }
  }
}
```

#### run アクション

```
コマンドを実行し、on_error 設定に従って処理:
- ignore: 常に exit 0（エラーを無視、デフォルト）
- fail: exit 2、stderr にエラーメッセージを出力

working_dir オプション:
- デフォルト: ${file_dir}（file_path の親ディレクトリ）
- 相対パスの場合: ${workspace_root} からの相対パスとして解決
- 変数展開対応: ${file_dir}, ${workspace_root} などを使用可能
```

#### log アクション

```
exit code: 0
ログを log_file に出力（log_file は必須）
```

#### マッチなし

```
exit code: 0
stdout: (空)
stderr: (空)
```

## Exit Code 一覧

| Exit Code | 意味 |
|-----------|------|
| 0 | 正常終了（マッチなし、transform 成功、run 成功、log 成功） |
| 1 | 内部エラー（設定ファイルパースエラー、JSON パースエラー、正規表現エラー） |
| 2 | ブロック（block アクション、run で on_error=fail） |

## 内部アーキテクチャ

### 動作フロー

```
1. stdin から Claude Code hook の JSON を受け取る
2. 設定ファイル (.claude/hooks-rules.toml) を読み込む
3. ルールを priority 順（降順）にソート
4. 各ルールを順番に評価し、最初にマッチしたルールを適用
5. アクションに応じた出力を生成
6. 適切な exit code で終了
```

### ルールマッチングアルゴリズム

1. `event` フィールドがコマンドライン引数と一致
2. `matcher` 正規表現が `tool_name` にマッチ
3. `when` 条件すべてを評価（AND 結合）
   - `when.command`: tool_input.command に対して正規表現マッチ
   - `when.file_path`: tool_input.file_path に対して正規表現マッチ
   - `when.branch`: 現在の Git ブランチと完全一致
4. すべての条件を満たす場合、ルールが適用される

### 変数展開の実装

単純な文字列置換で実装。展開順序は固定：

1. `${tool_name}` -> tool_name の値
2. `${command}` -> tool_input.command の値（存在する場合）
3. `${file_path}` -> tool_input.file_path の値（存在する場合）
4. `${file_dir}` -> file_path の親ディレクトリ（存在する場合）
5. `${workspace_root}` -> cchooked の CWD（カレントワーキングディレクトリ）
6. `${branch}` -> `git rev-parse --abbrev-ref HEAD` の出力

## モジュール構成

```
cchooked/
├── Cargo.toml
├── src/
│   ├── main.rs           # エントリーポイント、CLI 引数処理
│   ├── config.rs         # TOML 設定の読み込み・パース・バリデーション
│   ├── rule.rs           # ルール定義、マッチング評価ロジック
│   ├── action.rs         # 各アクションの実行（block, transform, run, log）
│   ├── context.rs        # 実行コンテキスト（変数、Git 情報取得）
│   ├── output.rs         # 出力フォーマット生成（JSON シリアライズ）
│   └── error.rs          # エラー型定義
├── tests/
│   ├── integration_tests.rs
│   └── fixtures/         # テスト用設定ファイル
└── docs/
    └── spec.md           # このファイル
```

### 各モジュールの責務

#### main.rs

- CLI 引数のパース（event 名、--config オプション）
- stdin からの JSON 読み込み
- 各モジュールの呼び出しとエラーハンドリング
- exit code の制御

#### config.rs

- TOML ファイルの読み込み
- 設定構造体へのデシリアライズ
- バリデーション（必須フィールドの確認、値の妥当性チェック）

#### rule.rs

- `Rule` 構造体の定義
- priority によるソート
- マッチング評価（event, matcher, when 条件）

#### action.rs

- `block`: stderr へのメッセージ出力
- `transform`: JSON 出力の生成
- `run`: 外部コマンド実行と結果処理
- `log`: ログファイル/stderr への出力

#### context.rs

- 入力 JSON からの値抽出
- Git ブランチ名の取得
- 変数展開処理

#### output.rs

- hookSpecificOutput JSON の構築
- シリアライズ

#### error.rs

- `CchookedError` enum の定義
- エラーメッセージのフォーマット

## エラーハンドリング

### エラー種別

| エラー | exit code | 処理 |
|--------|-----------|------|
| 設定ファイルが見つからない | 0 | 警告を stderr に出力、ルールなしとして続行 |
| 設定ファイルのパースエラー | 1 | エラーを stderr に出力して終了 |
| stdin の JSON パースエラー | 1 | エラーを stderr に出力して終了 |
| 正規表現の構文エラー | 1 | エラーを stderr に出力して終了 |
| log アクションで log_file 未指定 | 1 | エラーを stderr に出力して終了 |
| Git コマンド失敗 | - | branch を空文字として続行 |
| run コマンド失敗 | on_error 依存 | ignore: 0, fail: 2 |

### エラーメッセージフォーマット

```
cchooked: error: {エラー種別}: {詳細}
```

例：
```
cchooked: error: config parse error: expected string at line 5, column 10
cchooked: error: invalid regex in rule 'my-rule': unclosed group
```

## 実装優先順位

### Phase 1: コア機能

1. stdin JSON パース
2. TOML 設定読み込み
3. ルール評価（when.command のみ）
4. block アクション
5. 適切な exit code

### Phase 2: 基本アクション

6. transform アクション
7. when.file_path 条件

### Phase 3: 拡張機能

8. run アクション
9. log アクション
10. when.branch 条件（git コマンド実行）
11. priority ソート
12. 変数展開

### Phase 4: 仕上げ

13. エラーハンドリング改善
14. テスト追加
15. ドキュメント
16. GitHub Actions でリリース自動化

## テストケース

### 1. block アクション

**入力:**
```json
{"tool_name": "Bash", "tool_input": {"command": "npm install express"}}
```

**設定:**
```toml
[rules.no-npm]
event = "PreToolUse"
matcher = "Bash"
action = "block"
message = "use bun"
when.command = "^npm\\s"
```

**期待:**
- exit code: 2
- stderr: "use bun"

### 2. transform アクション

**入力:**
```json
{"tool_name": "Bash", "tool_input": {"command": "npm install express"}}
```

**設定:**
```toml
[rules.npm-to-bun]
event = "PreToolUse"
matcher = "Bash"
action = "transform"
when.command = "^npm\\s"
transform.command = ["^npm", "bun"]
```

**期待:**
- exit code: 0
- stdout: `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","updatedInput":{"command":"bun install express"}}}`

### 3. マッチなし

**入力:**
```json
{"tool_name": "Bash", "tool_input": {"command": "bun install express"}}
```

**設定:** 上記と同じ

**期待:**
- exit code: 0
- stdout/stderr: 空

### 4. priority 順序

**設定:**
```toml
[rules.low]
priority = 1
event = "PreToolUse"
matcher = "Bash"
action = "block"
message = "low"
when.command = ".*"

[rules.high]
priority = 10
event = "PreToolUse"
matcher = "Bash"
action = "block"
message = "high"
when.command = ".*"
```

**期待:** "high" がマッチ

### 5. when 条件の AND 評価

**入力:**
```json
{"tool_name": "Write", "tool_input": {"file_path": "/src/index.ts"}}
```

**設定:**
```toml
[rules.protect-src-on-main]
event = "PreToolUse"
matcher = "Write"
action = "block"
message = "cannot edit src on main"
when.branch = "main"
when.file_path = "^/src/.*"
```

**期待（main ブランチの場合）:**
- exit code: 2
- stderr: "cannot edit src on main"

**期待（feature ブランチの場合）:**
- exit code: 0
- stdout/stderr: 空

### 6. run アクションの on_error

**設定:**
```toml
[rules.lint]
event = "PostToolUse"
matcher = "Write"
action = "run"
command = "eslint ${file_path}"
on_error = "fail"
when.file_path = ".*\\.js$"
```

**期待（eslint が失敗した場合）:**
- exit code: 2
- stderr: eslint のエラーメッセージ

## 技術スタック

- 言語: Rust (2024 edition)
- 依存クレート:
  - `serde` / `serde_json` - JSON 入出力
  - `toml` - 設定ファイルパース
  - `regex` - 正規表現マッチング
  - `chrono` - タイムスタンプ（log アクション用）
  - `glob` - ファイルパスパターン（オプション）
