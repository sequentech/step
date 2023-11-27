ui = true

storage "file" {
    path = "/vault"
}

listener "tcp" {
    address = "0.0.0.0:8201"
    tls_disable = true
}
