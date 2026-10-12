-- templated: __splink__filtered_neighbours_2
-- physical: __splink__filtered_neighbours_2_bd8fe2972
CREATE TABLE __splink__filtered_neighbours_2_bd8fe2972 AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__filtered_neighbours_1_6b167cb0c as n
left join __splink__representatives_2_4c791c9e9 as l
on l.node_id = n.node_id
left join __splink__representatives_2_4c791c9e9 as r
on n.neighbour = r.node_id
where l.representative <> r.representative
