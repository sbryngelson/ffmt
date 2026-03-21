SUBROUTINE s_test()
#:call GPU_HOST_DATA(use_device_addr='[buff]')
CALL MPI_Send(buff,n,mpi_p,dst,tag,comm,ierr)
DO i=1,n
x(i)=y(i)
END DO
#:endcall GPU_HOST_DATA
END SUBROUTINE s_test
