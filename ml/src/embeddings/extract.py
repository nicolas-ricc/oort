import embeddings.TextEmbedder as Embedder

def extract_embeddings(text = "Argentina's economy faces challenges such as inflation, poverty, and political instability."):
    embedder = Embedder.TextEmbedder()
    embeddings = embedder.get_embeddings(text)
    return embeddings