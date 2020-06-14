#!/bin/sh
docker run --rm --name db -v $PWD/data:/data/db -d mongo:4.0 --smallfiles --replSet rs0 --oplogSize 128
sleep 5
docker exec -ti db mongo --eval "printjson(rs.initiate())"
docker run --rm --name rocketchat -p 3000:3000 --link db --env MONGO_OPLOG_URL=mongodb://db:27017/local -d rocket.chat
