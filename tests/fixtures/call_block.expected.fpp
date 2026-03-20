subroutine s_test()
    #:call GPU_HOST_DATA(use_device_addr='[buff]')
        call MPI_Send(buff, n, mpi_p, dst, tag, comm, ierr)
        do i = 1, n
            x(i) = y(i)
        end do
    #:endcall GPU_HOST_DATA
end subroutine s_test
