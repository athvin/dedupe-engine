-- templated: __splink__filtered_neighbours_1
-- physical: __splink__filtered_neighbours_1_a35ddb34a
CREATE TABLE __splink__filtered_neighbours_1_a35ddb34a AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__df_neighbours_0b0a752fc as n
left join __splink__representatives_1_5a5252b1a as l
on l.node_id = n.node_id
left join __splink__representatives_1_5a5252b1a as r
on n.neighbour = r.node_id
where l.representative <> r.representative
