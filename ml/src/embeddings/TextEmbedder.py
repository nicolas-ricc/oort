from transformers import AutoTokenizer, AutoModel
import torch

class TextEmbedder:
    def __init__(self, model_name='bert-base-uncased'):
        self.tokenizer = AutoTokenizer.from_pretrained(model_name)
        self.model = AutoModel.from_pretrained(model_name)

    def get_embeddings(self, text):
        # Tokenize and encode text
        inputs = self.tokenizer(text, return_tensors='pt', truncation=True, padding=True, max_length=512)
        with torch.no_grad():
            outputs = self.model(**inputs)
        # Use the last hidden state as embeddings
        embeddings = outputs.last_hidden_state.mean(dim=1).squeeze()
        return embeddings
