#!/bin/sh

# Start the Ollama server in the background
ollama serve &

# Store the PID of the server
SERVER_PID=$!

# Pull the required models
echo "Pulling mxbai-embed-large model..."
ollama pull mxbai-embed-large
echo "Pulling cognitivetech/obook_summary model..."
ollama pull cognitivetech/obook_summary

# Keep the container running by waiting for the server process
echo "Models pulled, Ollama is ready"