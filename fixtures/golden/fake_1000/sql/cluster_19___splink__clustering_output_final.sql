-- templated: __splink__clustering_output_final
-- physical: __splink__clustering_output_final_4a5c69b2a
CREATE TABLE __splink__clustering_output_final_4a5c69b2a AS
select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_1_8b08be219

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_2_9438e3906

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_3_cc76071fd

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_unstable_3_586420928
