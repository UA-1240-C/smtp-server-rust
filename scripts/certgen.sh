# Define paths for certificates and keys
CERT_DIR=../server/certs
KEY_DIR=../server/certs
CERT_FILE=$CERT_DIR/server.crt
KEY_FILE=$KEY_DIR/server.key

# Create certificates and keys if they don't exist
if [ ! -f "$CERT_FILE" ] || [ ! -f "$KEY_FILE" ]; then
    echo "Generating self-signed certificates..."

    # Create directories if they do not exist
    sudo mkdir -p $CERT_DIR
    sudo mkdir -p $KEY_DIR

    # Generate a private key without password
        sudo openssl genpkey -algorithm RSA -out $KEY_FILE

        # Generate Diffie-Hellman parameters (you can choose a different size, e.g., 4096)
        # sudo openssl dhparam -out $DH_PARAM_FILE 2048

        # Generate a self-signed certificate
        sudo openssl req -new -x509 -key $KEY_FILE -out $CERT_FILE -days 365 -subj "/C=US/ST=State/L=City/O=Organization/OU=Unit/CN=localhost"

    chmod +rw $CERT_FILE
    chmod +rw $KEY_FILE
fi
