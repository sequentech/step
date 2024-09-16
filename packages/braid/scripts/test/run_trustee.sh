# you must set this value
STEP_HOME=.
/usr/bin/time -f "%E real\t%M kb\t%P cpu\t%U user\t%S sys" $STEP_HOME/packages/target/release/main_m --server-url http://127.0.0.1:50051 --trustee-config trustee.toml --no-cache
