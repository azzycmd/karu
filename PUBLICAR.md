# Publicar o Karu de graca

O servidor aceita conexoes externas e le a porta pelo ambiente:

```bash
KARU_HOST=0.0.0.0 KARU_PORT=8765 python3 server.py
```

Em hospedagens como Render, a mesma porta tambem responde `GET`/`HEAD` com
`Karu online`, entao os health checks HTTP nao quebram o WebSocket.

O cliente desktop conecta por padrao em `wss://karu-tx61.onrender.com`.
Local continua simples:

```bash
cargo run
```

Para trocar o servidor manualmente, use `KARU_WS_URL`:

```bash
KARU_WS_URL=wss://SEU-SERVIDOR cargo run
```

Para usar a UI mais fluida, prefira build release:

```bash
KARU_WS_URL=wss://SEU-SERVIDOR cargo run --release
```

Se ainda travar, rode sem o painel lateral pesado:

```bash
KARU_UI_LITE=1 KARU_WS_URL=wss://SEU-SERVIDOR cargo run --release
```

## Diagnostico

Teste primeiro o health check HTTP do servidor:

```bash
curl -I https://SEU-SERVIDOR.onrender.com/
```

Tem que voltar `HTTP/2 200` ou `HTTP/1.1 200`.

Depois teste uma versao Rust reduzida, sem UI:

```bash
cargo run --bin karu_probe
```

Para testar login tambem:

```bash
KARU_USER=seu_usuario KARU_PASS=sua_senha cargo run --bin karu_probe
```

Se o probe conecta e a UI nao, o problema esta no cliente desktop. Se o probe
nao conecta, o problema esta na URL, no deploy ou no servidor.

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
