module m
    
    
    IMPLICIT NONE
    
    

    TYPE(vector_field), ALLOCATABLE, DIMENSION(:) :: q_cons_ts !<
    !! Cell-average conservative variables at each time-stage (TS)
    
    TYPE(scalar_field), PRIVATE, ALLOCATABLE, DIMENSION(:) :: q_prim_vf !<
    !! Cell-average primitive variables at the current time-stage
    

    TYPE(scalar_field), ALLOCATABLE, DIMENSION(:) :: rhs_vf !<
    !! Cell-average RHS variables at the current time-stage
    

    TYPE(vector_field), ALLOCATABLE, DIMENSION(:) :: q_prim_ts !<
    !! Cell-average primitive variables at consecutive TIMESTEPS


    INTEGER, PRIVATE :: num_ts !<
    !! Number of time stages in the time-stepping scheme
    
    
    CONTAINS
end module m
