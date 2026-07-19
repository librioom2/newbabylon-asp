import gguf

# Пути к файлам
model_path = 'models/multilingual-e5-small-F16.gguf'
output_path = 'e5_tokens.txt'

print(f"📖 Читаю токены из {model_path}...")

reader = gguf.GGUFReader(model_path)
# Достаем поле с токенами
tokens_field = reader.get_field('tokenizer.ggml.tokens')

if tokens_field is None:
    print("❌ Ошибка: Поле 'tokenizer.ggml.tokens' не найдено в GGUF!")
    exit(1)

with open(output_path, 'w', encoding='utf-8') as f:
    for i, token_data in enumerate(tokens_field.parts):
        # Превращаем данные в байты, затем в строку
        try:
            # GGUF хранит токены как массив байтов
            raw_bytes = bytes(token_data)
            token_str = raw_bytes.decode('utf-8', errors='ignore')
            # Записываем в формате слово : ID
            f.write(f"{token_str} : {i}\n")
        except Exception as e:
            continue

print(f"✅ Готово! Сохранено {len(tokens_field.parts)} токенов в {output_path}")
