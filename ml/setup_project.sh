#!/bin/bash

# Nombre del proyecto
PROJECT_NAME="text_embedding_backend"

# Crear la estructura de carpetas
mkdir -p $PROJECT_NAME/{data,models,src,tests}

# Crear archivos esenciales
touch $PROJECT_NAME/src/{__init__.py,embeddings.py,file_processor.py,main.py}
touch $PROJECT_NAME/tests/test_embeddings.py
touch $PROJECT_NAME/requirements.txt
touch $PROJECT_NAME/.env
touch $PROJECT_NAME/.gitignore
touch $PROJECT_NAME/README.md

# Mensaje de éxito
echo "Estructura de proyecto $PROJECT_NAME creada con éxito."

