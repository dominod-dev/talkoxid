FROM nginx:latest
RUN DEBIAN_FRONTEND=noninteractive apt-get update && apt-get install -y ssl-cert
COPY nginx.conf /etc/nginx/nginx.conf
