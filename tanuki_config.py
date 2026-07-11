#!/usr/bin/env python3
import os
import sys
import subprocess

# ==========================================
# 0. 依存パッケージの自動チェックと導入ガード
# ==========================================
def check_dependencies():
    # 接続テストで llm_manager が内部で httpx を使用するため、httpx のみチェック
    packages = ["httpx"]
    missing = []
    for pkg in packages:
        try:
            __import__(pkg)
        except ImportError:
            missing.append(pkg)
            
    if missing:
        print(f"🐾 T.A.N.U.K.I. Config: 必要なパッケージ {missing} が見つかりませんわ。")
        print("  自動インストールを実行します...")
        try:
            subprocess.check_call([sys.executable, "-m", "pip", "install"] + missing)
            print("  ✓ インストールが完了いたしましたわ！")
        except Exception as e:
            print(f"  ❌ パッケージの自動インストールに失敗しました: {e}")
            print(f"  手動で 'pip install {' '.join(missing)}' を実行してください。")
            sys.exit(1)

check_dependencies()

# ==========================================
# 1. llm_manager 共通ライブラリの読み込み
# ==========================================
LLM_MANAGER_PATH = os.path.abspath(os.path.join(os.path.dirname(__file__), "../llm_manager"))
if os.path.exists(LLM_MANAGER_PATH):
    sys.path.append(os.path.dirname(LLM_MANAGER_PATH))
else:
    print(f"⚠️ Warning: llm_manager library not found at: {LLM_MANAGER_PATH}")

try:
    from llm_manager import ChatModelConfig, ModelProvider, ModelCapabilities, ModelRegistry, LLMClient
except ImportError as ex:
    print(f"❌ llm_manager のインポートに失敗しました: {ex}")
    print("llm_manager ライブラリが正しい位置にあるか確認してくださいわ🐾")
    sys.exit(1)

# Windows環境での絵文字文字化け即死防止
if sys.platform == "win32":
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8')

# ==========================================
# 2. .env 操作ロジック
# ==========================================
ENV_FILE = ".env"

def load_env():
    env_vars = {}
    if not os.path.exists(ENV_FILE):
        return {
            "TANUKI_API_BASE": "http://localhost:3001",
            "TANUKI_MODEL": "gemma4:e4b",
            "TANUKI_MODELS_CONFIG": "config/models_config.json",
            "TANUKI_NO_REDUCE": "1",
            "TANUKI_TARGET_DIRS": "../Documents/Archive/Devlog,../Documents/Archive/Specifications,../Documents/Archive/Media,../Documents/Technical,../Documents/01_Projects,../Documents/Active"
        }
    with open(ENV_FILE, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                parts = line.split("=", 1)
                if len(parts) == 2:
                    key = parts[0].strip()
                    val = parts[1].strip()
                    if val.startswith(('"', "'")) and val.endswith(('"', "'")):
                        val = val[1:-1]
                    env_vars[key] = val
    return env_vars

def save_env(env_vars):
    with open(ENV_FILE, "w", encoding="utf-8") as f:
        f.write("# ==========================================\n")
        f.write("# T.A.N.U.K.I. Environment Configuration\n")
        f.write("# ==========================================\n\n")
        for k, v in env_vars.items():
            f.write(f"{k}={v}\n")

# ==========================================
# 3. 軽量対話UI用ヘルパー関数群 (標準ライブラリ)
# ==========================================
def prompt_choice(message, choices):
    """番号付きでリストを表示し、ユーザーに選択させます。"""
    print(f"\n[?] {message}")
    for i, choice in enumerate(choices):
        print(f"  {i + 1}) {choice}")
    
    while True:
        try:
            val = input("👉 番号を入力してください: ").strip()
            if not val:
                continue
            idx = int(val) - 1
            if 0 <= idx < len(choices):
                return choices[idx]
            else:
                print(f"❌ 1 から {len(choices)} の範囲で入力してくださいわ🐾")
        except ValueError:
            print("❌ 有効な数字を入力してくださいわ🐾")

def prompt_text(message, default=None):
    """テキスト入力を促します。"""
    prompt_str = f"👉 {message}"
    if default is not None:
        prompt_str += f" [デフォルト: {default}]"
    prompt_str += ": "
    
    val = input(prompt_str).strip()
    if not val and default is not None:
        return default
    return val

def prompt_confirm(message, default=False):
    """Yes/No の確認を求めます。"""
    default_str = "Y/n" if default else "y/N"
    while True:
        val = input(f"👉 {message} ({default_str}): ").strip().lower()
        if not val:
            return default
        if val.startswith('y'):
            return True
        if val.startswith('n'):
            return False
        print("❌ 'y' または 'n' で入力してくださいわ🐾")

def prompt_checkbox(message, choices, default_checked=None):
    """擬似的な複数選択（チェックボックス風）メニューを実装します。"""
    if default_checked is None:
        default_checked = []
    
    checked = set(default_checked)
    while True:
        print(f"\n[?] {message} (番号を入力してトグル、[決定]で確定)")
        menu_choices = []
        for choice_label, choice_value in choices:
            status = "[x]" if choice_value in checked else "[ ]"
            menu_choices.append(f"{status} {choice_label}")
        menu_choices.append("[決定]")
        
        for i, mc in enumerate(menu_choices):
            print(f"  {i + 1}) {mc}")
            
        try:
            val = input("👉 番号を入力してください: ").strip()
            if not val:
                continue
            idx = int(val) - 1
            if idx == len(choices): # [決定]
                break
            elif 0 <= idx < len(choices):
                target_val = choices[idx][1]
                if target_val in checked:
                    checked.remove(target_val)
                else:
                    checked.add(target_val)
            else:
                print("❌ 範囲外の番号です。")
        except ValueError:
            print("❌ 数字を入力してください。")
            
    return list(checked)

# ==========================================
# 4. 各機能の UI 実装
# ==========================================

def manage_env():
    env_vars = load_env()
    while True:
        choices = [f"{k} = {v}" for k, v in env_vars.items()] + ["[戻る]"]
        selection = prompt_choice("編集する環境変数を選択してください", choices)
        
        if selection == "[戻る]":
            break
            
        key = selection.split(" = ", 1)[0]
        current_val = env_vars[key]
        
        new_val = prompt_text(f"{key} の新しい値を入力してください", default=current_val)
        env_vars[key] = new_val
        save_env(env_vars)
        print(f"  ✓ {key} を更新して保存しましたわ！💮\n")

def get_registry():
    env_vars = load_env()
    config_path = env_vars.get("TANUKI_MODELS_CONFIG", "config/models_config.json")
    return ModelRegistry(config_path)

def manage_models():
    while True:
        registry = get_registry()
        models = registry.list_models()
        
        choices = []
        for m in models:
            choices.append(f"[{m.provider.value}] {m.model_name} (表示名: {m.display_name or 'なし'})")
        choices += ["[新規追加]", "[戻る]"]
        
        selection = prompt_choice("登録済みモデル一覧 (編集・追加するモデルを選択してください)", choices)
        
        if selection == "[戻る]":
            break
        elif selection == "[新規追加]":
            add_new_model(registry)
        else:
            idx = choices.index(selection)
            selected_model = models[idx]
            edit_model(registry, selected_model)

def add_new_model(registry):
    print("\n--- 🤖 新しいカスタムモデルの登録 ---")
    
    # 1. プロバイダーの選択
    providers = [p.value for p in ModelProvider]
    provider_val = prompt_choice("LLMプロバイダーを選択してください", providers)
    provider = ModelProvider(provider_val)
    
    # 2. 基本情報の入力
    model_name = prompt_text("モデル識別名 (例: gemini-1.5-flash)")
    if not model_name:
        print("  ⚠️ モデル識別名は必須ですわ。")
        return
        
    display_name = prompt_text("表示名 (例: Gemini Flash API)", default=model_name)
    base_url = prompt_text("Base URL (OllamaやLocalの場合のみ必要、省略可)", default="")
    api_key = prompt_text("API Key (クラウドAPIの場合に必要、省略可)", default="")
    
    # 3. 機能の選択
    caps_choices = [
        ("Reasoning (思考プロセスモデル)", "reasoning"),
        ("Vision (画像認識)", "vision"),
        ("Websearch (ウェブ探索)", "websearch"),
        ("CORS", "cors"),
        ("CURL", "curl")
    ]
    caps_list = prompt_checkbox("サポートする機能を選択してください", caps_choices)
    
    capabilities = ModelCapabilities(
        reasoning="reasoning" in caps_list,
        vision="vision" in caps_list,
        websearch="websearch" in caps_list,
        cors="cors" in caps_list,
        curl="curl" in caps_list
    )
    
    config = ChatModelConfig(
        model_name=model_name,
        provider=provider,
        display_name=display_name or None,
        base_url=base_url or None,
        api_key=api_key or None,
        capabilities=capabilities
    )
    
    registry.add_model(config)
    print(f"  ✓ モデル '{config.model_name}' を追加保存しましたわ！💮\n")

def edit_model(registry, model_config):
    while True:
        print(f"\n--- 🤖 モデル編集: {model_config.model_name} ---")
        choices = [
            f"表示名: {model_config.display_name or '未設定'}",
            f"Base URL: {model_config.base_url or '未設定'}",
            f"API Key: {'********' if model_config.api_key else '未設定'}",
            f"Capabilities: {model_config.capabilities.to_dict()}",
            "[削除する]",
            "[戻る]"
        ]
        
        selection = prompt_choice("変更するプロパティを選択してください", choices)
        
        if selection == "[戻る]":
            break
        elif selection == "[削除する]":
            confirm = prompt_confirm(f"本当にモデル '{model_config.model_name}' を削除してよろしいですか？", default=False)
            if confirm:
                registry.remove_model(model_config.model_name)
                print(f"  ✓ モデルを削除しましたわ🐾")
                break
        else:
            prop_idx = choices.index(selection)
            if prop_idx == 0:  # Display Name
                val = prompt_text("新しい表示名", default=model_config.display_name)
                model_config.display_name = val or None
            elif prop_idx == 1:  # Base URL
                val = prompt_text("新しいBase URL", default=model_config.base_url)
                model_config.base_url = val or None
            elif prop_idx == 2:  # API Key
                val = prompt_text("新しいAPI Key", default=model_config.api_key)
                model_config.api_key = val or None
            elif prop_idx == 3:  # Capabilities
                current_caps = model_config.capabilities
                checked = []
                if current_caps.reasoning: checked.append("reasoning")
                if current_caps.vision: checked.append("vision")
                if current_caps.websearch: checked.append("websearch")
                if current_caps.cors: checked.append("cors")
                if current_caps.curl: checked.append("curl")
                
                caps_choices = [
                    ("Reasoning", "reasoning"),
                    ("Vision", "vision"),
                    ("Websearch", "websearch"),
                    ("CORS", "cors"),
                    ("CURL", "curl")
                ]
                new_caps = prompt_checkbox("サポートする機能の編集", caps_choices, default_checked=checked)
                model_config.capabilities = ModelCapabilities(
                    reasoning="reasoning" in new_caps,
                    vision="vision" in new_caps,
                    websearch="websearch" in new_caps,
                    cors="cors" in new_caps,
                    curl="curl" in new_caps
                )
                    
            registry.add_model(model_config)
            print("  ✓ モデル設定を更新しましたわ！💮")

def test_connection():
    registry = get_registry()
    models = registry.list_models()
    if not models:
        print("\n⚠️ 登録されているモデルがありません。先にモデルを登録してくださいわ🐾\n")
        return
        
    choices = [f"[{m.provider.value}] {m.model_name}" for m in models] + ["[戻る]"]
    selection = prompt_choice("接続テストを行うモデルを選択してください", choices)
    
    if selection == "[戻る]":
        return
        
    idx = choices.index(selection)
    config = models[idx]
    
    if config.provider == ModelProvider.GEMINI and not config.api_key:
        fallback_key = os.getenv("GEMINI_API_KEY") or os.getenv("GOOGLE_API_KEY")
        if fallback_key:
            config.api_key = fallback_key
            
    print(f"\n🔌 接続テスト中: {config.model_name} (Provider: {config.provider.value})...")
    
    try:
        client = LLMClient(config)
        messages = [
            {"role": "user", "content": "Hello! Respond in exactly 3 words."}
        ]
        
        if config.base_url:
            print(f"  Target Base URL: {config.base_url}")
            
        print("  Waiting for response...")
        response = client.chat(messages)
        print(f"\n✅ 接続成功！ 応答:\n>>> \"{response.strip()}\"\n")
    except Exception as e:
        print(f"\n❌ 接続に失敗しましたわ: {e}")
        print("  APIキーやベースURLの設定が正しいか確認してくださいわ🐾\n")
    
    input("Press Enter to continue...")

def main_menu():
    while True:
        print("\n" + "=" * 50)
        print("🐾 T.A.N.U.K.I. System Configuration TUI (v1.0) 🐾")
        print("=" * 50)
        
        choices = [
            "TANUKI 環境変数設定 (.env)",
            "LLM モデル設定の管理 (models_config.json)",
            "LLM 接続テスト (Test Connection)",
            "終了"
        ]
        
        selection = prompt_choice("実行したいメニューを選択してください", choices)
        
        if "環境変数" in selection:
            manage_env()
        elif "モデル設定" in selection:
            manage_models()
        elif "接続テスト" in selection:
            test_connection()
        elif "終了" in selection:
            print("\nご主人様、設定を完了いたしました！行ってらっしゃいませ🐾💮")
            break

if __name__ == "__main__":
    try:
        main_menu()
    except KeyboardInterrupt:
        print("\n\n設定操作が中断されましたわ。またね！🐾")
