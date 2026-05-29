import asyncio
import websockets
import json
import os
import bcrypt  # Biblioteca para hash de senhas seguro
from datetime import datetime

ARQUIVO_HISTORICO = "chat.json"
ARQUIVO_USUARIOS = "usuarios.json"  # Novo arquivo para guardar contas protegidas
LIMITE_HISTORICO_POR_CANAL = 200
INTERVALO_SALVAR_HISTORICO = 1.0

canais = {
    "geral": {"users": set(), "historico": []},
    "dev": {"users": set(), "historico": []},
    "ajuda": {"users": set(), "historico": []}
}

clients = {}
usuarios_cadastrados = {}  # { "username": {"email": "...", "password_hash": "...", "pfp": "..."} }
historico_sujo = False

def carregar_usuarios():
    global usuarios_cadastrados
    if os.path.exists(ARQUIVO_USUARIOS) and os.path.getsize(ARQUIVO_USUARIOS) > 0:
        try:
            with open(ARQUIVO_USUARIOS, "r", encoding="utf-8") as f:
                dados = json.load(f)
                usuarios_cadastrados = dados
            print("[SERVER] Banco de dados de usuários carregado com segurança.")
        except Exception as e:
            print(f"[SERVER] Arquivo corrompido ou inválido ({e}). Iniciando base limpa.")
            usuarios_cadastrados = {}
    else:
        # Se o arquivo não existir ou tiver 0 bytes, inicia o dict vazio com segurança
        print("[SERVER] Nenhum banco de usuários encontrado. Criando um novo.")
        usuarios_cadastrados = {}

def salvar_usuarios_no_disco():
    dados_serializaveis = {}
    for user, info in usuarios_cadastrados.items():
        password_hash = info["password_hash"]
        if isinstance(password_hash, bytes):
            password_hash = password_hash.decode("utf-8")

        dados_serializaveis[user] = {
            "email": info.get("email", ""),
            "password_hash": password_hash,
            "pfp": info.get("pfp", ""),
        }

    with open(ARQUIVO_USUARIOS, "w", encoding="utf-8") as f:
        json.dump(dados_serializaveis, f, indent=4, ensure_ascii=False)

def salvar_usuario_no_disco(username, email, password_puro):
    # Gera o sal (salt) e o hash da senha usando bcrypt
    salt = bcrypt.gensalt()
    pw_hash = bcrypt.hashpw(password_puro.encode('utf-8'), salt)

    usuarios_cadastrados[username] = {
        "email": email,
        "password_hash": pw_hash.decode('utf-8'), # Salva como string no JSON
        "pfp": "",
    }

    salvar_usuarios_no_disco()

def verificar_senha(username, password_puro):
    if username not in usuarios_cadastrados:
        return False
    
    hash_salvador = usuarios_cadastrados[username]["password_hash"]
    
    # PROTEÇÃO: Se o hash estiver como string (texto), converte para bytes antes de checar
    if isinstance(hash_salvador, str):
        hash_salvador = hash_salvador.encode('utf-8')
        
    # Compara a senha digitada com o hash criptografado
    return bcrypt.checkpw(password_puro.encode('utf-8'), hash_salvador)

# --- SISTEMA DE HISTÓRICO ---
def carregar_historico_do_disco():
    if os.path.exists(ARQUIVO_HISTORICO):
        try:
            with open(ARQUIVO_HISTORICO, "r", encoding="utf-8") as f:
                dados_salvos = json.load(f)
                for canal, msgs in dados_salvos.items():
                    if canal in canais:
                        canais[canal]["historico"] = msgs[-LIMITE_HISTORICO_POR_CANAL:]
        except Exception as e:
            print(f"[SERVER] Erro ao ler histórico: {e}")

def salvar_historico_no_disco():
    dados_para_salvar = {canal: info["historico"] for canal, info in canais.items()}
    with open(ARQUIVO_HISTORICO, "w", encoding="utf-8") as f:
        json.dump(dados_para_salvar, f, ensure_ascii=False, separators=(",", ":"))

def marcar_historico_sujo():
    global historico_sujo
    historico_sujo = True

async def salvador_historico_periodico():
    global historico_sujo
    while True:
        await asyncio.sleep(INTERVALO_SALVAR_HISTORICO)
        if historico_sujo:
            salvar_historico_no_disco()
            historico_sujo = False

async def enviar_lista_usuarios(canal):
    if canal in canais:
        lista_usuarios = []
        for ws in canais[canal]["users"]:
            if ws in clients:
                username = clients[ws]["username"]
                lista_usuarios.append({
                    "username": username,
                    "pfp": usuarios_cadastrados.get(username, {}).get("pfp", ""),
                })
        payload = json.dumps({"type": "user_list", "users": lista_usuarios})
        for ws in canais[canal]["users"]:
            try: await ws.send(payload)
            except: pass

async def broadcast_para_canal(canal, payload, remetente_ws=None):
    if canal in canais:
        for client in canais[canal]["users"]:
            if client != remetente_ws:
                try: await client.send(payload)
                except: pass

# --- MANIPULADOR DE CONEXÕES ---
async def chat_handler(websocket):
    clients[websocket] = {"username": "random", "canal": "geral", "autenticado": False, "pfp": ""}
    canais["geral"]["users"].add(websocket)
    
    try:
        async for message in websocket:
            data = json.loads(message)
            user_info = clients[websocket]
            
            # --- NOVO: REGISTRO DE CONTA ---
            if data["type"] == "register":
                username = data["username"]
                email = data["email"]
                password = data["password"]
                
                if username in usuarios_cadastrados:
                    await websocket.send(json.dumps({"type": "auth_response", "success": False, "msg": "Usuário já existe!"}))
                else:
                    salvar_usuario_no_disco(username, email, password)
                    print(f"[SERVER] Nova conta criada: {username} ({email})")
                    await websocket.send(json.dumps({"type": "auth_response", "success": True, "msg": "Conta criada! Faça login."}))
                continue

            # --- CORRIGIDO: LOGIN COM VALIDAÇÃO DE SENHA ---
            if data["type"] == "login":
                username = data["username"]
                password = data["password"]
                
                if verificar_senha(username, password):
                    user_info["username"] = username
                    user_info["pfp"] = usuarios_cadastrados.get(username, {}).get("pfp", "")
                    user_info["autenticado"] = True
                    print(f"\n[SERVER] Autenticado com sucesso: {username}")
                    
                    # Envia resposta de sucesso para o cliente destravar a tela
                    await websocket.send(json.dumps({"type": "auth_response", "success": True, "msg": "Login aceito"}))
                    
                    # Envia o histórico
                    for msg_antiga in canais["geral"]["historico"]:
                        await websocket.send(json.dumps(msg_antiga))
                    
                    await enviar_lista_usuarios("geral")
                    boas_vindas = json.dumps({"type": "system", "msg": f"{username} entrou no chat!"})
                    await broadcast_para_canal("geral", boas_vindas, websocket)
                else:
                    await websocket.send(json.dumps({"type": "auth_response", "success": False, "msg": "Senha ou usuário incorretos!"}))
                continue

            # Bloqueia qualquer engraçadinho que tente mandar msg sem logar
            if not user_info["autenticado"]:
                continue

            # --- MUDANÇA DE CANAL ---
            if data["type"] == "join_channel":
                canal_antigo = user_info["canal"]
                novo_canal = data["channel"]
                
                if novo_canal not in canais:
                    canais[novo_canal] = {"users": set(), "historico": []}
                if canal_antigo in canais and websocket in canais[canal_antigo]["users"]:
                    canais[canal_antigo]["users"].remove(websocket)
                    await enviar_lista_usuarios(canal_antigo)
                
                canais[novo_canal]["users"].add(websocket)
                user_info["canal"] = novo_canal
                
                for msg_antiga in canais[novo_canal]["historico"]:
                    await websocket.send(json.dumps(msg_antiga))
                await enviar_lista_usuarios(novo_canal)
                continue

            # --- ATUALIZAÇÃO DE FOTO DE PERFIL ---
            if data["type"] == "pfp_update":
                username = user_info["username"]
                pfp = data.get("pfp", "")

                if (
                    not isinstance(pfp, str)
                    or not pfp.startswith("data:image/")
                    or ";base64," not in pfp
                    or len(pfp) > 1_400_000
                ):
                    await websocket.send(json.dumps({"type": "system", "msg": "PFP inválida ou grande demais."}))
                    continue

                usuarios_cadastrados.setdefault(username, {})["pfp"] = pfp
                user_info["pfp"] = pfp
                salvar_usuarios_no_disco()

                await websocket.send(json.dumps({"type": "system", "msg": "PFP salva no servidor."}))
                await enviar_lista_usuarios(user_info["canal"])
                continue

            # --- MENSAGEM DE CHAT ---
            if data["type"] == "chat":
                canal_atual = user_info["canal"]
                msg_payload = {
                    "type": "chat",
                    "user": user_info["username"],
                    "msg": data["msg"],
                    "pfp": usuarios_cadastrados.get(user_info["username"], {}).get("pfp", ""),
                    "time": datetime.now().strftime("%H:%M"),
                }
                
                canais[canal_atual]["historico"].append(msg_payload)
                if len(canais[canal_atual]["historico"]) > LIMITE_HISTORICO_POR_CANAL:
                    excedente = len(canais[canal_atual]["historico"]) - LIMITE_HISTORICO_POR_CANAL
                    del canais[canal_atual]["historico"][:excedente]
                marcar_historico_sujo()
                
                print(f"\r\x1b[2K[#{canal_atual}][{user_info['username']}]: {data['msg']}")
                print("Mensagem global > ", end="", flush=True)
                
                await broadcast_para_canal(canal_atual, json.dumps(msg_payload))
                await asyncio.sleep(0)

    except websockets.exceptions.ConnectionClosed:
        pass
    finally:
        if websocket in clients:
            user_info = clients[websocket]
            canal_atual = user_info["canal"]
            if canal_atual in canais and websocket in canais[canal_atual]["users"]:
                canais[canal_atual]["users"].remove(websocket)
            del clients[websocket]
            if user_info["autenticado"]:
                await enviar_lista_usuarios(canal_atual)

async def main():
    carregar_usuarios()
    carregar_historico_do_disco()
    host = os.environ.get("KARU_HOST", "0.0.0.0")
    porta = int(os.environ.get("KARU_PORT", os.environ.get("PORT", "8765")))
    tarefa_salvar = asyncio.create_task(salvador_historico_periodico())
    try:
        async with websockets.serve(chat_handler, host, porta, ping_interval=20):
            print(f"Servidor Karu Seguro rodando em ws://{host}:{porta}")
            await asyncio.Future()
    finally:
        tarefa_salvar.cancel()
        if historico_sujo:
            salvar_historico_no_disco()

if __name__ == "__main__":
    asyncio.run(main())
