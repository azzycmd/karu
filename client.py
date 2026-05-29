import asyncio
import websockets
import json
import sys
import os

class Colors:
    SERVER = "\033[93m"  # Amarelo
    USER = "\033[94m"    # Azul
    PROMPT = "\033[92m"  # Verde
    RESET = "\033[0m"

# Estado local do cliente para renderizar o prompt corretamente
state = {
    "canal_atual": "geral"
}

async def receive_messages(websocket):
    try:
        async for message in websocket:
            data = json.loads(message)
            
            if data["type"] == "chat":
                print("\r\x1b[2K", end="")
                print(f"[{Colors.USER}{data['user']}{Colors.RESET}]: {data['msg']}")
                print(f"{Colors.PROMPT}[{state['canal_atual']}] > {Colors.RESET}", end="", flush=True)
                
            elif data["type"] == "system":
                print("\r\x1b[2K", end="")
                print(f"{Colors.SERVER}[*] {data['msg']}{Colors.RESET}")
                print(f"{Colors.PROMPT}[{state['canal_atual']}] > {Colors.RESET}", end="", flush=True)
                
            elif data["type"] == "channel_list":
                print("\r\x1b[2K", end="")
                print(f"{Colors.SERVER}[*] Canais ativos: {', '.join(data['channels'])}{Colors.RESET}")
                print(f"{Colors.PROMPT}[{state['canal_atual']}] > {Colors.RESET}", end="", flush=True)
    except asyncio.CancelledError:
        pass
    except:
        print(f"\n{Colors.SERVER}Conexão com o servidor perdida.{Colors.RESET}")

async def send_messages(websocket, username):
    # Envia o login
    await websocket.send(json.dumps({"type": "login", "username": username}))
    
    loop = asyncio.get_event_loop()
    try:
        while True:
            prompt_str = f"{Colors.PROMPT}[{state['canal_atual']}] > {Colors.RESET}"
            msg = await loop.run_in_executor(None, input, prompt_str)
            
            if not msg:
                continue
                
            # --- PARSER DE COMANDOS CLI ---
            if msg.startswith("/"):
                partes = msg.split(" ", 1)
                comando = partes[0].lower()
                
                if comando == "/sair":
                    print("Saindo...")
                    os._exit(0)
                    
                elif comando == "/canais":
                    # Pede a lista de canais para o servidor
                    await websocket.send(json.dumps({"type": "get_channels"}))
                    
                elif comando == "/join" and len(partes) > 1:
                    novo_canal = partes[1].strip()
                    state["canal_atual"] = novo_canal
                    await websocket.send(json.dumps({"type": "join_channel", "channel": novo_canal}))
                    print(f"{Colors.SERVER}[*] Mudando para o canal #{novo_canal}...{Colors.RESET}")
                    
                elif comando == "/ajuda":
                    print(f"\n{Colors.SERVER}--- COMANDOS DISPONÍVEIS ---")
                    print("/canais         - Lista os canais existentes")
                    print("/join <nome>    - Entra ou cria um canal")
                    print("/sair           - Fecha o chat")
                    print(f"/ajuda          - Mostra essa lista{Colors.RESET}\n")
                else:
                    print(f"{Colors.SERVER}[*] Comando desconhecido. Digite /ajuda{Colors.RESET}")
            else:
                # Envia mensagem normal para o canal atual
                await websocket.send(json.dumps({"type": "chat", "msg": msg}))
                
    except asyncio.CancelledError:
        pass

async def main():
    username = input("Digite seu username: ")
    uri = "ws://localhost:8765"
    try:
        async with websockets.connect(uri) as websocket:
            await asyncio.gather(
                receive_messages(websocket), 
                send_messages(websocket, username)
            )
    except asyncio.CancelledError:
        pass

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nSaindo do chat... Até logo!")
        os._exit(0)