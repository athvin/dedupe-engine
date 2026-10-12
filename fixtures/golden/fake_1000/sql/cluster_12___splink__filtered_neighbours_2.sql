-- templated: __splink__filtered_neighbours_2
-- physical: __splink__filtered_neighbours_2_027dc0ffc
CREATE TABLE __splink__filtered_neighbours_2_027dc0ffc AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__filtered_neighbours_1_a35ddb34a as n
left join __splink__representatives_2_2c7f43183 as l
on l.node_id = n.node_id
left join __splink__representatives_2_2c7f43183 as r
on n.neighbour = r.node_id
where l.representative <> r.representative
