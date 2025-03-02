import json
from dimensionality import clusterConcepts
from concepts.model import ConceptsModel
from embeddings.model import EmbeddingModel
from pathlib import Path
import numpy as np

class NumpyEncoder(json.JSONEncoder):
    def default(self, obj):
        if isinstance(obj, np.ndarray):
            return obj.tolist()
        if isinstance(obj, np.integer):
            return int(obj)
        if isinstance(obj, np.floating):
            return float(obj)
        return super(NumpyEncoder, self).default(obj)

def extractClusteredConcepts():
    current_dir = Path(__file__).parent
    file_path = current_dir / "mocks/firstTale.txt"
    with open(file_path, "r") as file:
        text = file.read()
        model = ConceptsModel()
        concepts = model.generate_concepts(text)
        print("Concepts:", concepts)

        embeddingsModel = EmbeddingModel()
        embeddings = embeddingsModel.get_batch_embeddings(concepts)
        print("Embeddings:", embeddings)
        
        return clusterConcepts(concepts, embeddings)
    
def dumpClusteredConcepts(clustered_concepts):
    current_dir = Path(__file__).parent
    file_path = current_dir / "mocks/clusteredConcepts.json"
    with open(file_path, "w") as file:
        json.dump(clustered_concepts, file, cls=NumpyEncoder)

clustered_concepts = extractClusteredConcepts()
dumpClusteredConcepts(clustered_concepts)
