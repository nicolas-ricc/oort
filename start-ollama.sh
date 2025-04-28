#!/bin/sh

# Start the Ollama server in the background
ollama serve &

# Store the PID of the server
SERVER_PID=$!

# Pull the required models
echo "Pulling snowflake-arctic-embed2 model..."
ollama pull snowflake-arctic-embed2
echo "Pulling phi3.5 model..."
ollama pull phi3.5

# Keep the container running by waiting for the server process
echo "Models pulled, Ollama is ready"