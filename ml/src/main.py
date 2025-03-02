import json
from dimensionality import clusterConcepts
from concepts.model import ConceptsModel
from embeddings.model import EmbeddingModel
import numpy as np
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

# Initialize models
conceptsModel = ConceptsModel()
embeddingsModel = EmbeddingModel()

app = FastAPI()

class TextInput(BaseModel):
    text: str

class NumpyEncoder(json.JSONEncoder):
    def default(self, obj):
        if isinstance(obj, np.ndarray):
            return obj.tolist()
        if isinstance(obj, np.integer):
            return int(obj)
        if isinstance(obj, np.floating):
            return float(obj)
        return super(NumpyEncoder, self).default(obj)

@app.post("/api/process_text")
async def process_text(input_data: TextInput):
    try:
        concepts = conceptsModel.generate_concepts(input_data.text)
        
        if not concepts:
            raise HTTPException(status_code=422, detail="No concepts could be extracted from the provided text")
        
        embeddings = embeddingsModel.get_batch_embeddings(concepts)
        
        if not embeddings or len(embeddings) != len(concepts):
            raise HTTPException(status_code=500, detail="Error generating embeddings")
        
        # Cluster concepts with embeddings
        clustered_results = clusterConcepts(concepts, embeddings)
        
        return {
            "success": True,
            "data": clustered_results
        }
    
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)