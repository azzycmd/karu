import random

# Banco de dados atualizado e corrigido
vocab = [
    {"r": "Клавиатура", "p": "Teclado", "g": "f", "pron": "Klaviaturá"},
    {"r": "Процессор", "p": "Processador", "g": "m", "pron": "Pratséssar"},
    {"r": "Монитор", "p": "Monitor", "g": "m", "pron": "Manitór"},
    {"r": "Плата", "p": "Placa-mãe", "g": "f", "pron": "Pláta"},
    {"r": "Компьютер", "p": "Computador", "g": "m", "pron": "Kampyúter"},
    {"r": "Система", "p": "Sistema", "g": "f", "pron": "Sistyéma"},
    {"r": "Код", "p": "Código", "g": "m", "pron": "Kod"},
    {"r": "Экран", "p": "Tela", "g": "m", "pron": "Ekrán"} # Corrigido!
]

frases_dia_a_dia = [
    {"p": "Eu não entendo", "r": "Я не понимаю", "pron": "Ya nye panimáyu"},
    {"p": "Onde está o código?", "r": "Где код?", "pron": "Gdyé kod?"},
    {"p": "Eu sei", "r": "Я знаю", "pron": "Ya znáyu"},
    {"p": "O que é isso?", "r": "Что это?", "pron": "Shtó éta?"},
    {"p": "Isto não é o meu sistema", "r": "Это не моя система", "pron": "Éta nye mayá sistyéma"}
]

def quiz_palavras():
    print("\n--- MODO: TRADUÇÃO DE HARDWARE ---")
    random.shuffle(vocab)
    for item in vocab:
        resp = input(f"Como se diz '{item['p']}'? ({item['pron']}): ").strip().lower()
        if resp == item['r'].lower():
            print("✅ Correto!")
        else:
            print(f"❌ Errou. O correto é: {item['r']}")

def quiz_frases_hardware():
    print("\n--- MODO: CONSTRUÇÃO (HARDWARE) ---")
    print("Dica: 'Это мой/моя...' (O travessão — é opcional, mas elegante!)")
    random.shuffle(vocab)
    for item in vocab:
        posse = "мой" if item['g'] == 'm' else "моя"
        pronom = "он" if item['g'] == 'm' else "она"
        print(f"\nDesafio: 'Isto é {posse} {item['p']}. {pronom} está aqui.'")
        
        resp = input("Sua resposta: ").strip().lower()
        # Validação flexível que aceita com ou sem travessão
        if item['r'].lower() in resp and posse in resp and pronom in resp and "здесь" in resp:
            print(f"✅ Compilado com sucesso!")
        else:
            print(f"❌ Erro de sintaxe. Sugestão: 'Это {posse} {item['r'].lower()}. {pronom} здесь.'")

def quiz_dia_a_dia():
    print("\n--- MODO: SOBREVIVÊNCIA (DIA A DIA) ---")
    random.shuffle(frases_dia_a_dia)
    for frase in frases_dia_a_dia:
        print(f"\nTraduza: '{frase['p']}'")
        resp = input(f"Dica de som ({frase['pron']}): ").strip().lower()
        if resp == frase['r'].lower().replace("?", ""): # ignora interrogação na checagem
            print("✅ Perfeito!")
        else:
            print(f"❌ Erro. O correto é: {frase['r']}")

def main():
    print("=== MYCELIUM OS: RUSSIAN MODULE v0.2 ===")
    print("1. Tradução de Hardware")
    print("2. Construção de Frases (Hardware + Gênero)")
    print("3. Frases do Dia a Dia")
    
    op = input("\nEscolha o modo: ")
    if op == "1": quiz_palavras()
    elif op == "2": quiz_frases_hardware()
    elif op == "3": quiz_dia_a_dia()
    else: print("Modo inválido.")

if __name__ == "__main__":
    main()