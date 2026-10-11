-- templated: __splink__df_representatives
-- physical: __splink__df_representatives_0bfbeb6fd
CREATE TABLE __splink__df_representatives_0bfbeb6fd AS
select distinct
    node_id as node_id,
    node_id as representative,
    0 as stable
from __splink__df_nodes_with_composite_ids_974156385
