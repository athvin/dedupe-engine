-- templated: __splink__agreement_pattern_counts
-- physical: __splink__agreement_pattern_counts_7a8b3c86a
CREATE TABLE __splink__agreement_pattern_counts_7a8b3c86a AS
select
gamma_first_name,
gamma_surname,
gamma_city,
gamma_email,
count(*) as agreement_pattern_count
from __splink__df_comparison_vectors_9c7a8fb66
group by gamma_first_name, gamma_surname, gamma_city, gamma_email
