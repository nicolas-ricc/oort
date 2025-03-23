import json
import src.dimensionality as dimensionality 
from src.concepts.model import ConceptsModel
from src.embeddings.model import EmbeddingModel
import numpy as np
from fastapi import FastAPI, HTTPException, UploadFile, File, Form
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

# Initialize models
conceptsModel = ConceptsModel()
embeddingsModel = EmbeddingModel()

app = FastAPI()

# Add CORS middleware to allow frontend requests
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # Allow all origins in development
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

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

@app.post("/api/vectorize")
async def process_text(input_data: TextInput):
    try:
        concepts = conceptsModel.generate_concepts(input_data.text)
        
        if not concepts:
            raise HTTPException(status_code=422, detail="No concepts could be extracted from the provided text")
        
        embeddings = embeddingsModel.get_batch_embeddings([c["concept"] for c in concepts])
        
        if not embeddings or len(embeddings) != len(concepts):
            raise HTTPException(status_code=500, detail="Error generating embeddings")
        
        # Cluster concepts with embeddings
        clustered_results = dimensionality.clusterConcepts(concepts, embeddings)
        
        return {
            "success": True,
            "data": clustered_results
        }
    
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/api/upload")
async def upload_file(file: UploadFile = File(...)):
    try:
        # Read the file content
        content = await file.read()
        text = content.decode("utf-8")
        
        # Process the text content
        concepts = conceptsModel.generate_concepts(text)
        
        if not concepts:
            raise HTTPException(status_code=422, detail="No concepts could be extracted from the provided file")
        
        # Get embeddings for the concepts
        embeddings = embeddingsModel.get_batch_embeddings([c["concept"] for c in concepts])
        
        if not embeddings or len(embeddings) != len(concepts):
            raise HTTPException(status_code=500, detail="Error generating embeddings")
        
        # Cluster concepts with embeddings
        clustered_results = dimensionality.clusterConcepts(concepts, embeddings)
        
        return {
            "success": True,
            "data": clustered_results
        }
    
    except UnicodeDecodeError:
        raise HTTPException(status_code=422, detail="The file could not be decoded as text. Please upload a valid text file.")
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))