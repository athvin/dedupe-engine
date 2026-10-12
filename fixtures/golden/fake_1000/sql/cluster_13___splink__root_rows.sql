-- templated: __splink__root_rows
-- physical: __splink__root_rows_b9209e01f
CREATE TABLE __splink__root_rows_b9209e01f AS
select count(*) as count_of_edges_needing_processing
from __splink__filtered_neighbours_2_027dc0ffc
