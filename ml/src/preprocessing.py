import spacy
import re
import unicodedata

nlp = spacy.load("es_core_news_md")

# Función de normalización
def normalize_text(text):
    # Arreglar texto corrupto y normalizar unicode
    text = unicodedata.normalize('NFKD', text).encode('ascii', 'ignore').decode('utf-8')
    # Quitar caracteres especiales y múltiple espacio
    text = re.sub(r'[^\w\s]', '', text)
    text = re.sub(r'\s+', ' ', text).strip()
    return text.lower()

# Preprocesamiento completo
def preprocess_text(_text):

    # Normalizar texto
    print("Entering normalization")
    text = normalize_text(_text)
    print("Normalized text length:", len(text))
    nlp.max_length = 20000000  # Ajusta según el tamaño de tu texto

    # Procesar con SpaCy
    doc = nlp(text)
    
    # Eliminar stopwords y puntuación usando SpaCy y NLTK
    tokens = [
        token.lemma_ for token in doc
        if not token.is_punct  # Puntuación
    ]
    print("tokens::", tokens)
    return tokens