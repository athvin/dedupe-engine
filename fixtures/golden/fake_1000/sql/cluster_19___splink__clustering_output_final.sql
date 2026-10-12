-- templated: __splink__clustering_output_final
-- physical: __splink__clustering_output_final_c9af3f826
CREATE TABLE __splink__clustering_output_final_c9af3f826 AS
select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_1_9928214b7

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_2_f1235e7a8

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_3_135ff2687

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_unstable_3_6a5a58561
