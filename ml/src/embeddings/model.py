import requests
import numpy as np
from typing import List, Union
import json
import re
import spacy

class EmbeddingModel:
    def __init__(self, base_url: str = "http://localhost:11434"):
        self.base_url = base_url
        self.model_name = "mxbai-embed-large"
        # Configuramos URLs para diferentes endpoints
        self.embedding_url = f"{base_url}/api/embeddings"
        self.generate_url = f"{base_url}/api/generate"


    def get_batch_embeddings(self, texts: List[str]) -> List[np.ndarray]:
        """
        Generates embeddings for multiple texts.
        """
        embeddings = []
        
        # Verificamos que texts sea una lista
        if not isinstance(texts, list):
            print(f"Error: se esperaba una lista de textos, pero se recibió {type(texts)}")
            return []

        for text in texts:
            # Verificamos y limpiamos cada texto
            if not isinstance(text, str):
                text = str(text)
            
            # Eliminamos espacios en blanco extras y caracteres especiales
            text = text.strip()
            
            # Solo procesamos textos no vacíos
            if text:
                print(f"Processing: '{text[:50]}...'")  # Debug
                embedding = self.get_contextual_embeddings(text)
                if embedding is not None:
                    embeddings.append(embedding)
            else:
                print("This is an empty text, skipping...")  # Debug
                
        return embeddings

    def get_contextual_embeddings(self, text: str) -> np.ndarray:
        """
        Generates embeddings for a complete text.
        """
        if not text or not isinstance(text, str):
            print(f"Error: invalid text of type:  {type(text)}")
            return None

        payload = {
            "model": self.model_name,
            "prompt": text
        }

        try:
            # Debug info
            print(f"Sending request to: {self.embedding_url}")
            print(f"Payload: {json.dumps(payload, ensure_ascii=False)}")
            
            response = requests.post(
                self.embedding_url,
                json=payload,
                headers={'Content-Type': 'application/json'},
                timeout=30
            )
            
            if response.status_code != 200:
                print(f"Error {response.status_code}: {response.text}")
                return None
                
            embedding_data = response.json()
            print(embedding_data)  # Debug
            if "embedding" in embedding_data:
                return np.array(embedding_data['embedding'])
            else:
                print(f"Embedding response is in wrong format: {embedding_data}")
                return None
            
        except requests.exceptions.RequestException as e:
            print(f"HTTP Error: {str(e)}")
            if hasattr(e, 'response') and e.response is not None:
                print(f"Server response: {e.response.text}")
            return None

    def get_similarity(self, embedding1: np.ndarray, embedding2: np.ndarray) -> float:
        """
        Calculates cosine similarity between two embeddings.
        
        Args:
            embedding1: First embedding
            embedding2: Second embedding
            
        Returns:
            float: Cosine similarity between embeddings
        """
        # Normalize vectors
        norm1 = np.linalg.norm(embedding1)
        norm2 = np.linalg.norm(embedding2)
        
        if norm1 == 0 or norm2 == 0:
            return 0
            
        # Calculate cosine similarity
        return np.dot(embedding1, embedding2) / (norm1 * norm2)

    def find_most_similar(self, 
        query_embedding: np.ndarray, 
        comparison_embeddings: List[np.ndarray], 
        top_k: int = 5) -> List[tuple]:
        """
        Finds the most similar embeddings to a given one.
        
        Args:
            query_embedding: Query embedding to compare against
            comparison_embeddings: List of embeddings to compare with
            top_k: Number of similar results to return
            
        Returns:
            List[tuple]: List of (index, similarity) tuples sorted by similarity
        """
        similarities = []
        
        for i, emb in enumerate(comparison_embeddings):
            similarity = self.get_similarity(query_embedding, emb)
            similarities.append((i, similarity))
            
        # Sort by similarity in descending order
        similarities.sort(key=lambda x: x[1], reverse=True)
        
        return similarities[:top_k]
