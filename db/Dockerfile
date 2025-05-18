FROM cassandra:latest

# Copy CQL script to the image
COPY data.cql /docker-entrypoint-initdb.d/data.cql

# Create a script to execute the CQL file
RUN echo '#!/bin/bash \n\
echo "Waiting for Cassandra to start..." \n\
until cqlsh -e "describe keyspaces"; do \n\
  echo "Cassandra not ready - waiting..." \n\
  sleep 5 \n\
done \n\
echo "Cassandra ready - initializing database" \n\
cqlsh -f /docker-entrypoint-initdb.d/data.cql \n\
echo "Database initialized"' > /docker-entrypoint-initdb.d/init-db.sh \
&& chmod +x /docker-entrypoint-initdb.d/init-db.sh

# Set entrypoint to Cassandra with auto-initialization
ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["cassandra", "-f"]