import json
import re
import spacy
from ollama import Client

class ConceptsModel:
    def __init__(self, base_url="http://localhost:11434"):
        self.base_url = base_url
        # Initialize the Ollama client - but don't include the http:// prefix that's handled internally
        if base_url.startswith("http://"):
            base_url = base_url[7:]
        elif base_url.startswith("https://"):
            base_url = base_url[8:]
        self.client = Client(host=base_url)
        # Load spaCy model
        self.nlp = spacy.load('en_core_web_sm')
        
    def clean_text(self, text):
        """Cleans text from punctuation and special characters."""
        # Keep apostrophes for words like don't, won't, etc.
        text = re.sub(r'[^\w\s\']', ' ', text)
        # Remove apostrophes if not part of contractions
        text = re.sub(r'\s\'|\'\s', ' ', text)
        # Remove multiple spaces
        text = re.sub(r'\s+', ' ', text)
        return text.strip()
    
    def lemmatize_concept(self, concept):
        """Lemmatizes a concept maintaining phrase structure."""
        # First clean the text
        concept = self.clean_text(concept)
        print("LOGGING: Cleaned concept:", concept)
        doc = self.nlp(concept)
        lemmatized_words = []
        
        for token in doc:
            if token.pos_ == 'VERB':
                # Convert verbs to nouns using morphology
                # If ends in 'ing', convert to base form
                if token.text.endswith('ing'):
                    lemmatized_words.append(token.lemma_)
                # If it's a past tense verb, try to convert to noun
                elif token.tag_ == 'VBD':
                    # Use lemma as base and add common noun suffixes
                    base = token.lemma_
                    if base.endswith('e'):
                        lemmatized_words.append(base[:-1] + 'ation')
                    else:
                        lemmatized_words.append(base + 'ation')
                else:
                    # For other verbs, use base form
                    lemmatized_words.append(token.lemma_)
            else:
                # For non-verbs, use lemma directly
                lemmatized_words.append(token.lemma_)
        
        # Join words and clean extra spaces
        result = ' '.join(lemmatized_words).strip()
        return result

    def generate_concepts(self, text, model="phi3.5"):
        system_prompt = """You are a concept extractor that MUST:
        1. Extract key concepts from the text
        2. Output ONLY simple concepts separated by commas (NO bullet points, NO descriptions)
        4. Example output:
            Happy Prince, Golden Statue, Ruby Sword, Sapphire Eyes, Town Councillors
        
        DO NOT include:
        - Bullet points (-)
        - Descriptions or explanations
        - Newlines
        - Colons or semicolons"""

        template = f"Extract 5-10 key concepts from this text as simple words or short phrases separated by commas ONLY: {text[:500]}..."
        
        try:
            print(f"Requesting concepts using ollama client with model: {model}")
            
            # Use the client with proper parameters according to the ollama client docs
            response = self.client.generate(
                model=model,
                prompt=template,
                system=system_prompt,
                options={"temperature": 0}
            )
            
            # The response is a dictionary, not a requests.Response object
            content = response.get('response', '').strip()
            
            print("Content received:", content)
            
            # Clean the content
            content = (content
                .replace('\n', '')
                .replace('- ', '')
                .replace(': ', ', ')
            )
            
            # Process and lemmatize concepts
            concepts = []
            for concept in content.split(','):
                concept = concept.strip()
                if concept and len(concept.split()) <= 3:
                    # Lemmatize the concept
                    lemmatized = self.lemmatize_concept(concept)
                    concepts.append({"concept": lemmatized})
            
            print("LOGGING: Lemmatized concepts:", concepts)
            return concepts
                
        except Exception as e:
            print(f"Request error: {e}")
            return []