-- templated: __splink__clustering_output_final
-- physical: __splink__clustering_output_final_6cc04ee9c
CREATE TABLE __splink__clustering_output_final_6cc04ee9c AS
select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_1_73c5d3ac1

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_2_49e4ffd92

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_3_d149b866f

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_unstable_3_ada1b3123
