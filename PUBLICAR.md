# Publicar o Karu de graca

O servidor aceita conexoes externas e le a porta pelo ambiente:

```bash
KARU_HOST=0.0.0.0 KARU_PORT=8765 python3 server.py
```

Em hospedagens como Render, a mesma porta tambem responde `GET`/`HEAD` com
`Karu online`, entao os health checks HTTP nao quebram o WebSocket.

O cliente desktop usa `KARU_WS_URL`. Local continua igual:

```bash
cargo run
```

Para conectar em um servidor publico:

```bash
KARU_WS_URL=wss://SEU-SERVIDOR cargo run
```

## Render Free

1. Suba este projeto para um repositorio GitHub.
2. No Render, crie um Web Service novo apontando para o repositorio.
3. Use:
   - Runtime: Python
   - Build command: `pip install -r requirements.txt`
   - Start command: `python3 server.py`
   - Instance type: Free
4. Depois do deploy, copie a URL publica do Render e rode o cliente com:

```bash
KARU_WS_URL=wss://NOME-DO-SERVICO.onrender.com cargo run
```

Observacoes:

- O plano gratis pode dormir depois de inatividade. A primeira conexao depois disso pode demorar.
- `chat.json` e `usuarios.json` no disco local do Render nao sao uma base persistente confiavel para producao. Para um chat publico serio, o proximo passo e trocar esses JSON por um banco gratis.
