-- templated: __splink__agreement_pattern_counts
-- physical: __splink__agreement_pattern_counts_ffd6833ba
CREATE TABLE __splink__agreement_pattern_counts_ffd6833ba AS
select
gamma_first_name,
gamma_surname,
gamma_city,
gamma_email,
count(*) as agreement_pattern_count
from __splink__df_comparison_vectors_63fc1c277
group by gamma_first_name, gamma_surname, gamma_city, gamma_email
