import requests
import json

def test_endpoint(text, url="http://localhost:8000/api/process_text"):
    """
    Test the text processing endpoint
    
    Args:
        text: Text to process
        url: Endpoint URL
        
    Returns:
        The server's responsed
    """
    payload = {"text": text}
    
    response = requests.post(
        url,
        json=payload,
        headers={'Content-Type': 'application/json'}
    )
    
    if response.status_code == 200:
        return response.json()
    else:
        print(f"Error: {response.status_code}")
        print(response.text)
        return None

if __name__ == "__main__":
    # Sample text for testing
    sample_text = """
    The Happy Prince was a beautiful statue that stood high above the city. 
    He was covered with gold, had sapphires for eyes, and a large ruby on his sword. 
    One day, a little Swallow decided to rest at the statue's feet. 
    The Prince asked the Swallow to help distribute his gold leaves and jewels to the poor people in the city.
    """
    
    result = test_endpoint(sample_text)
    if result:
        print(json.dumps(result, indent=2))