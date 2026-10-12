-- templated: __splink__df_representatives
-- physical: __splink__df_representatives_89ac0e434
CREATE TABLE __splink__df_representatives_89ac0e434 AS
select distinct
    node_id as node_id,
    node_id as representative,
    0 as stable
from __splink__df_nodes_with_composite_ids_b4ae02e7f
