-- templated: __splink__root_rows
-- physical: __splink__root_rows_bd56ef057
CREATE TABLE __splink__root_rows_bd56ef057 AS
select count(*) as count_of_edges_needing_processing
from __splink__filtered_neighbours_1_6b167cb0c
