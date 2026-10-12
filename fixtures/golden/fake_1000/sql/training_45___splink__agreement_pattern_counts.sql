-- templated: __splink__agreement_pattern_counts
-- physical: __splink__agreement_pattern_counts_89773b53b
CREATE TABLE __splink__agreement_pattern_counts_89773b53b AS
select
gamma_first_name,
gamma_surname,
gamma_city,
gamma_email,
count(*) as agreement_pattern_count
from __splink__df_comparison_vectors_30a0a215c
group by gamma_first_name, gamma_surname, gamma_city, gamma_email
