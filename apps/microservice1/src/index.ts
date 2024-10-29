import express from 'express';
const app = express();
const port = process.env.PORT || 3001;

app.get('/', (req, res) => {
  res.json({ message: 'Microservice 1 is running' });
});

app.listen(port, () => {
  console.log(`Microservice 1 listening at http://localhost:${port}`);
});
