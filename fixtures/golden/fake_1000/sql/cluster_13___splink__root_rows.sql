-- templated: __splink__root_rows
-- physical: __splink__root_rows_5b5c3110a
CREATE TABLE __splink__root_rows_5b5c3110a AS
select count(*) as count_of_edges_needing_processing
from __splink__filtered_neighbours_2_bd8fe2972
