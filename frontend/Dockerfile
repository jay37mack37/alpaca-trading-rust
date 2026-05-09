# Use the official Node image
FROM node:20-slim as builder

WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm install

# Copy source
COPY . .

# Build the app
RUN npm run build

# Use Nginx to serve the build
FROM nginx:alpine

# Copy the build output to Nginx
COPY --from=builder /app/dist /usr/share/nginx/html

# Copy a simple Nginx config if needed, or use default
EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
