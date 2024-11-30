# redirs
This is a redis server clone inspired by Coding Challenges. The goal is to optimise to get closer to `redis-server` performance. 

## Benchmarking

Using `redis-benchmark -t SET,GET -q` as the benchmark:

##### `redis-server`: 
- SET: 92506.94 requests per second, p50=0.263 msec   
- GET: 94161.95 requests per second, p50=0.271 msec  

##### `redirs`:
- SET: 56148.23 requests per second, p50=0.471 msec   
- GET: 58997.05 requests per second, p50=0.447 msec

(On my dying i7 macbook with multiple other procs running)

## Optimisation ideas
- Minimise copying. Currently `Message`s own their data. This is not ideal for moving content between the network and database.
  - Write to DB directly from network buffer capture
  - Respond to network/user directly from DB reference. 
  - Both of these ^ will probably require breaking down responses into multiple steps...?
- Compress storage - currently number values are being stored in their string representation. This could probably be easily done with an `enum Value` datatype
- async IO instead of threads
- More direct `Message` parsing. The grammar of the Redis Serialisation Protocol (RESP) is straightforward and everything is id-symbol prefixed. The current implementation uses `nom` which makes handling complex grammars easier but may not be optimal for this use-case. 