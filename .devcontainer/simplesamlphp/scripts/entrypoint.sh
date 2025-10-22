#!/bin/sh

# Start memcached in the background.
# We'll run it as the www-data user, which already exists in the image.
memcached -d -u www-data

# Now, execute the original entrypoint script from the php:apache image.
# This will perform its setup and then start Apache in the foreground.
# "$@" passes along any command-line arguments (like the default 'apache2-foreground').
exec docker-php-entrypoint "$@"