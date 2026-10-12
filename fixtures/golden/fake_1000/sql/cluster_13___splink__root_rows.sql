-- templated: __splink__root_rows
-- physical: __splink__root_rows_1bf7f4c42
CREATE TABLE __splink__root_rows_1bf7f4c42 AS
select count(*) as count_of_edges_needing_processing
from __splink__filtered_neighbours_2_2669f7a2b
