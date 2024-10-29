import express from 'express';
const app = express();
const port = process.env.PORT || 3002;

app.get('/', (req, res) => {
  res.json({ message: 'Microservice 2 is running' });
});

app.listen(port, () => {
  console.log(`Microservice 2 listening at http://localhost:${port}`);
});
