-- templated: __splink__df_neighbours
-- physical: __splink__df_neighbours_55afdff2a
CREATE TABLE __splink__df_neighbours_55afdff2a AS
select
    node_id_l as node_rep,
    node_id_l as node_id,
    node_id_r as neighbour_rep,
    node_id_r as neighbour
from __splink__df_edges_from_predict_770712cfc
where match_probability >= 0.95

union all

select
    node_id_r as node_rep,
    node_id_r as node_id,
    node_id_l as neighbour_rep,
    node_id_l as neighbour
from __splink__df_edges_from_predict_770712cfc
where match_probability >= 0.95
