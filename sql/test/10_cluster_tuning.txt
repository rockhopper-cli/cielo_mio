-- system tuning - Cluster Hardware & Memory Profile Settings
ALTER SYSTEM SET io_method = 'worker'; 
ALTER SYSTEM SET io_combine_limit = 128; 
ALTER SYSTEM SET shared_buffers = '1GB'; 
ALTER SYSTEM SET work_mem = '64MB'; 
ALTER SYSTEM SET maintenance_work_mem = '256MB'; 
ALTER SYSTEM SET synchronous_commit = 'off'; 
ALTER SYSTEM SET random_page_cost = 1.1; 
ALTER SYSTEM SET effective_io_concurrency = 256; 
ALTER SYSTEM SET max_parallel_workers_per_gather = 4;

SELECT pg_reload_conf();