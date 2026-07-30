# AgentFreeze

**エージェントの依存管理外で、何が変わったかを見る。**

AgentFreezeはローカル専用・非実行型の証拠収集ツールです。Claude、Codex、Cursor、
Gemini、共通agentディレクトリの対応済みSkills・プラグイン・MCP定義を、起動せずに
検出します。現在の状態を`agentfreeze.lock`へ保存し、追加・削除・内容変更を見つけます。

```console
$ agentfreeze scan
Found 79 extensions: 34 skills, 26 plugins, 19 MCP servers

$ agentfreeze freeze
Frozen 79 extensions in agentfreeze.lock

$ agentfreeze verify
Verified 79 extensions — no drift

$ agentfreeze watch
Watching 79 user extensions every 60s — Ctrl-C to stop
```

この`79`はメンテナ1名の実測値であり、平均値ではありません。

## 30秒で試す

GitHub CLIの`gh attestation verify`を利用できるmacOS / Linux:

```sh
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/skikkh/agentfreeze/releases/latest/download/install.sh | sh

agentfreeze scan
agentfreeze freeze
agentfreeze verify
agentfreeze watch
```

installerはSHA-256を必ず確認し、既定ではGitHub CLIを使ってrepository・release workflow・
tag・commit・GitHub-hosted runnerまで拘束したbuild provenance検証も必須にします。
`gh`なしでsame-origin checksumだけを明示的に受容する手順とinstaller自体の検証手順は
[English README](README.md#install)にあります。

`freeze`は対応済みのユーザー領域だけを固定します。現在のリポジトリは
`agentfreeze scan --scope workspace`でinventoryできますが、project-localな認可にはSeal、
宣言済みのproject/global依存管理にはMicrosoft APMを使います。

## 何を保証するか

AgentFreezeは「安全判定スキャナ」ではありません。「今ある拡張が、以前固定したものと
完全に同じか」を検証します。

- パスは`$HOME` / `$WORKSPACE`へ正規化
- 内容はSHA-256で決定的にハッシュ
- `.git`を含む隠しファイルも除外せずhash
- MCPの生設定は保存せず、秘密らしいenv/header/query key・CLI flag・URL credentialをredact
- tenant/region/modeなど非秘密のrouting値はhashに含め、接続先の取り違えを検知
- 拡張内容のhashではsymlink先を辿らずリンク文字列をhash。manifest探索だけはroot内の
  directory aliasを深さ制限付きで解決し、循環・上限超過・境界外なら失敗
- `verify`はclean=0、drift=1、実行エラー=2

lockfileも変更できる攻撃者、初回導入時から悪性の拡張、実行時だけ変わる挙動は防げません。
未知の名前を持つ低エントロピーの秘密値はdigestへ影響し、周辺定義が既知ならオフライン推測の
対象になる可能性があります。公開前に拡張名とsource locationを確認してください。
詳しくは[脅威モデル](docs/threat-model.md)を参照してください。

## 既存ツールとの境界

[Seal](https://sealfile.dev/)はproject-localなagent bundleのlockfileとロード時強制を
すでに定義しています。Microsoft APMはprojectだけでなくglobal scopeの依存管理・
content-hash lockfile・audit・SBOM・CI policyを提供しています。Snyk Agent Scanは
ローカルsecurity scanと組織向けbackground monitoringを提供しています。

したがってAgentFreezeは「global対応だから新しい」とは主張しません。v0.1で検証するのは、
依存管理に宣言されていない、または管理外で書き換わった対応済みローカル状態を、拡張や
MCPサーバーを起動せずに証拠化できるか、という一点です。workspace entryはv1 lockfileへ
入れません。

## 現在地

v0.1ではinventory・freeze・verify・間隔指定のpolling monitorを提供します。これは
2週間の需要検証であり、標準化・事業化・買収可能性は未検証です。拡張自体のsigned origin、
native filesystem event、隔離環境での発火検証は未実装です。実装済みと誤解させる安全主張は
しません。release assetにはGitHub build provenanceを付けます。

ライセンスはMITです。仕様変更と中立運営の方針は
[GOVERNANCE.md](docs/governance.md)にあります。

[English README](README.md)
