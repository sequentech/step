FROM php:8.3-apache

# Simplify installing php extensions. See
COPY --from=mlocati/php-extension-installer /usr/bin/install-php-extensions /usr/bin/

# Install the various packages needed by ssp and composer
RUN apt-get update -y && apt-get -y upgrade && apt-get install -y \
    git \
    file \
    ssl-cert \
    patch \
    curl \
    zip \
    vim \
    wget \
    memcached \
    unzip && \
    apt-get clean && \
    apt-get autoclean

RUN install-php-extensions gmp ldap soap apcu memcached zip pdo_pgsql pdo_mysql intl @composer-2.8.12

WORKDIR /var
RUN wget https://github.com/simplesamlphp/simplesamlphp/releases/download/v2.4.3/simplesamlphp-2.4.3-full.tar.gz
RUN tar xzf simplesamlphp-2.4.3-full.tar.gz
RUN mv simplesamlphp-2.4.3 simplesamlphp

WORKDIR /var/simplesamlphp

# Use a real cert instead of snakeoil
# APACHE_CERT_NAME=ssl-cert-snakeoil
ENV APACHE_CERT_NAME=local-stack-dev \
    COMPOSER_HOME=/opt/simplesamlphp/.composer \
    SSP_ADMIN_PASSWORD=123 \
    SSP_APACHE_ALIAS="simplesaml/" \
    SSP_DELETE_MODULES="" \
    SSP_DIR=/var/simplesamlphp \
    SSP_ENABLED_MODULES="" \
    SSP_HASH=ff52cde9b3f90ff5dd76a97e67842462da2d8edc2128fbe8dcdbbd03b1b21945 \
    SSP_LOG_HANDLER=errorlog \
    SSP_LOG_LEVEL=6 \
    SSP_VERSION=2.4.2 \
    COMPOSER_REQUIRE=""

RUN  chown www-data $SSP_DIR && \
     mkdir -p /var/cache/simplesamlphp/ && chown www-data /var/cache/simplesamlphp/

RUN cp /var/simplesamlphp/config/config.php.dist /var/simplesamlphp/config/config.php \
    && echo 'require "config-override-base.php";' >> /var/simplesamlphp/config/config.php \
    && mkdir -p $SSP_DIR/data \
    && mkdir -p $SSP_DIR/ticketcache

RUN chown -R www-data:www-data /var/simplesamlphp/data
RUN chmod -R 775 /var/simplesamlphp/data

COPY scripts/entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/entrypoint.sh
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

CMD ["apache2-foreground"]