-- templated: __splink__filtered_neighbours_3
-- physical: __splink__filtered_neighbours_3_d1c95f725
CREATE TABLE __splink__filtered_neighbours_3_d1c95f725 AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__filtered_neighbours_2_bd8fe2972 as n
left join __splink__representatives_3_ca2b18b47 as l
on l.node_id = n.node_id
left join __splink__representatives_3_ca2b18b47 as r
on n.neighbour = r.node_id
where l.representative <> r.representative
