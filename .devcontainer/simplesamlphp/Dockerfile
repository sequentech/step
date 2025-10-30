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

ENV APACHE_CERT_NAME=ssl-cert-snakeoil \
    COMPOSER_HOME=/opt/simplesamlphp/.composer \
    SSP_ADMIN_PASSWORD=123 \
    SSP_APACHE_ALIAS="simplesaml/" \
    SSP_DELETE_MODULES="" \
    SSP_DIR=/var/simplesaml \
    SSP_ENABLED_MODULES="" \
    SSP_HASH=ff52cde9b3f90ff5dd76a97e67842462da2d8edc2128fbe8dcdbbd03b1b21945 \
    SSP_LOG_HANDLER=errorlog \
    SSP_LOG_LEVEL=6 \
    SSP_VERSION=2.4.2 \
    COMPOSER_REQUIRE=""

RUN cp $PHP_INI_DIR/php.ini-production $PHP_INI_DIR/php.ini \
    && echo 'expose_php = off' >> $PHP_INI_DIR/php.ini 

# Add default index page 
RUN echo 'By default SSP is on <a href="/simplesaml/">/simplesaml/</a>' > /var/www/html/index.html

# Enable Apache modules
RUN a2enmod ssl rewrite headers
EXPOSE 443

# Tell Apache to listen on port 8083
RUN echo "Listen 8083" >> /etc/apache2/ports.conf
EXPOSE 8083

# Allow ARM machines do run image
RUN echo "Mutex posixsem" >> /etc/apache2/apache2.conf

# Sites must end with .conf
ADD simplesaml.conf /etc/apache2/sites-available/simplesaml.conf
RUN a2ensite simplesaml
RUN  mkdir -p $SSP_DIR && chown www-data $SSP_DIR && \
     mkdir -p /var/cache/simplesamlphp/ && chown www-data /var/cache/simplesamlphp/ && \
     mkdir -p $COMPOSER_HOME && chown www-data $COMPOSER_HOME
# Composer installs seem to require us to run npm
RUN if [ -n "$SSP_COMPOSER_VERSION" ]; then apt-get update -y && apt-get install -y npm; \
    mkdir -p "/var/www/.npm" && chown www-data "/var/www/.npm" && chown -R 33:33 "/var/www/.npm"; fi && \
    apt-get clean && \
    apt-get autoclean

USER www-data
WORKDIR /tmp
RUN curl -L -o /tmp/ssp.tar.gz https://github.com/simplesamlphp/simplesamlphp/releases/download/v$SSP_VERSION/simplesamlphp-$SSP_VERSION-full.tar.gz
RUN echo "$SSP_HASH  /tmp/ssp.tar.gz" | sha256sum -c -
RUN tar xvzf /tmp/ssp.tar.gz --strip-components 1 -C $SSP_DIR
RUN echo $SSP_VERSION > $SSP_DIR/ssp_release_version

# Copy the config
ADD config/* $SSP_DIR/config/
RUN mv $SSP_DIR/config/config.php.dist $SSP_DIR/config/config.php
ADD metadata/* $SSP_DIR/metadata/
ADD cert/* $SSP_DIR/cert/

# Copy the idp initiated sso page
ADD public/idp-initiated-sso.php $SSP_DIR/public/idp-initiated-sso.php

RUN mkdir $SSP_DIR/data
RUN chmod -R 775 $SSP_DIR/data

USER root
COPY scripts/entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/entrypoint.sh
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

CMD ["apache2-foreground"]