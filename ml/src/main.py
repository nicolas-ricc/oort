import embeddings.extract as extract
import dimensionality
import numpy as np
embeddings = extract.extract_embeddings("Argentina's economy faces challenges such as inflation, poverty, and political instability.")
print("Generated Embeddings:", embeddings)
reduced_embeddings = dimensionality.reduce_to_3d([embeddings], method='pca')
print("Reduced Embeddings:", reduced_embeddings)
