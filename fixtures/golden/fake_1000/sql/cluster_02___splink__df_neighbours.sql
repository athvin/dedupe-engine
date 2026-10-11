-- templated: __splink__df_neighbours
-- physical: __splink__df_neighbours_3c8e6b280
CREATE TABLE __splink__df_neighbours_3c8e6b280 AS
select
    node_id_l as node_rep,
    node_id_l as node_id,
    node_id_r as neighbour_rep,
    node_id_r as neighbour
from __splink__df_edges_from_predict_3780ea389
where match_probability >= 0.95

union all

select
    node_id_r as node_rep,
    node_id_r as node_id,
    node_id_l as neighbour_rep,
    node_id_l as neighbour
from __splink__df_edges_from_predict_3780ea389
where match_probability >= 0.95
