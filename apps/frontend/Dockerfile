FROM node:23-alpine

WORKDIR /app

# Install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

# Copy package.json and pnpm-lock.yaml (if available)
COPY package.json pnpm-lock.yaml* ./

# Install dependencies
RUN rm -rf node_modules && pnpm install --frozen-lockfile

# Copy the rest of the code
COPY . .

# Expose port
EXPOSE 3000

# Start the app in development mode with hot reloading
CMD ["pnpm", "run", "dev", "--", "--host", "0.0.0.0"]