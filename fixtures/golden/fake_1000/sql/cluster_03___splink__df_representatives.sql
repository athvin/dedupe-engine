-- templated: __splink__df_representatives
-- physical: __splink__df_representatives_510d48981
CREATE TABLE __splink__df_representatives_510d48981 AS
select distinct
    node_id as node_id,
    node_id as representative,
    0 as stable
from __splink__df_nodes_with_composite_ids_bc7c4e0fd
